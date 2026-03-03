// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Semantic Kernel Planner Output Translation
//!
//! Translates Semantic Kernel planner output (plan steps) into Cognitive Task spawn requests
//! with full dependency DAG construction and execution constraints.
//!
//! SK Planner.CreatePlan() outputs a sequence of steps with data dependencies.
//! This module converts that into CT spawner directives with explicit task graphs,
//! enabling efficient kernel scheduling and parallel execution where possible.
//!
//! Sec 4.3: SK Planner → CT Spawner Translation
//! Sec 4.2: Dependency DAG Construction
//! Sec 4.2: Plan Validation

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use crate::{
    error::AdapterError,
    AdapterResult,
};

/// Plan step identifier type.
pub type StepId = u64;

/// Semantic Kernel planner step for translation.
/// Sec 4.3: SK Plan Step
#[derive(Debug, Clone)]
pub struct PlannerStep {
    /// Step sequence number (0-based)
    pub sequence: usize,
    /// Target plugin and function (format: "PluginName.FunctionName")
    pub function_ref: String,
    /// Input parameters mapping
    pub parameters: BTreeMap<String, String>,
    /// Optional output variable name
    pub output_var: Option<String>,
}

impl PlannerStep {
    /// Creates a new planner step.
    pub fn new(sequence: usize, function_ref: String) -> Self {
        PlannerStep {
            sequence,
            function_ref,
            parameters: BTreeMap::new(),
            output_var: None,
        }
    }
}

/// Data dependency edge in the task graph.
/// Sec 4.2: Dependency Edge
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Source task ID
    pub from_task: StepId,
    /// Target task ID
    pub to_task: StepId,
    /// Data variable being passed
    pub data_var: String,
}

/// Cognitive Task spawn request.
/// Sec 4.3: CT Spawn Request
#[derive(Debug, Clone)]
pub struct CtSpawnRequest {
    /// Unique request identifier
    pub request_id: String,
    /// Task to spawn
    pub task_id: StepId,
    /// Function reference
    pub function_ref: String,
    /// Input parameters for task execution
    pub inputs: BTreeMap<String, String>,
    /// Task dependency IDs
    pub depends_on: Vec<StepId>,
    /// Priority level (0-100, higher = more urgent)
    pub priority: u8,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

impl CtSpawnRequest {
    /// Creates a new spawn request.
    pub fn new(request_id: String, task_id: StepId, function_ref: String) -> Self {
        CtSpawnRequest {
            request_id,
            task_id,
            function_ref,
            inputs: BTreeMap::new(),
            depends_on: Vec::new(),
            priority: 50,
            timeout_ms: 30000,
        }
    }
}

/// Task dependency graph representation.
/// Sec 4.2: Dependency DAG
#[derive(Debug, Clone)]
pub struct TaskDag {
    /// Task ID to spawn request mapping
    pub tasks: BTreeMap<StepId, CtSpawnRequest>,
    /// Dependency edges in the graph
    pub edges: Vec<DependencyEdge>,
    /// Data variable flow mapping
    pub data_flow: BTreeMap<String, Vec<(StepId, StepId)>>,
}

impl TaskDag {
    /// Creates a new empty task DAG.
    pub fn new() -> Self {
        TaskDag {
            tasks: BTreeMap::new(),
            edges: Vec::new(),
            data_flow: BTreeMap::new(),
        }
    }

    /// Adds a task to the DAG.
    pub fn add_task(&mut self, task: CtSpawnRequest) {
        self.tasks.insert(task.task_id, task);
    }

    /// Adds a dependency edge between tasks.
    pub fn add_edge(&mut self, edge: DependencyEdge) {
        self.edges.push(edge);
    }

    /// Gets topologically sorted task execution order.
    /// Sec 4.2: Topological Sort
    pub fn topological_sort(&self) -> AdapterResult<Vec<StepId>> {
        let mut sorted = Vec::new();
        let mut visited = BTreeMap::new();
        let mut visiting = BTreeMap::new();

        for task_id in self.tasks.keys() {
            if !visited.contains_key(task_id) {
                self.visit_task(*task_id, &mut visited, &mut visiting, &mut sorted)?;
            }
        }

        Ok(sorted)
    }

    /// Depth-first visit for topological sort.
    fn visit_task(
        &self,
        task_id: StepId,
        visited: &mut BTreeMap<StepId, bool>,
        visiting: &mut BTreeMap<StepId, bool>,
        sorted: &mut Vec<StepId>,
    ) -> AdapterResult<()> {
        if visited.contains_key(&task_id) {
            return Ok(());
        }

        if visiting.contains_key(&task_id) {
            return Err(AdapterError::TranslationError(
                alloc::format!("Cyclic dependency detected for task {}", task_id),
            ));
        }

        visiting.insert(task_id, true);

        // Visit dependencies
        if let Some(task) = self.tasks.get(&task_id) {
            for dep_id in &task.depends_on {
                self.visit_task(*dep_id, visited, visiting, sorted)?;
            }
        }

        visiting.remove(&task_id);
        visited.insert(task_id, true);
        sorted.push(task_id);

        Ok(())
    }

    /// Validates the DAG for consistency.
    /// Sec 4.2: DAG Validation
    pub fn validate(&self) -> AdapterResult<()> {
        // Check all edges reference existing tasks
        for edge in &self.edges {
            if !self.tasks.contains_key(&edge.from_task) {
                return Err(AdapterError::TranslationError(
                    alloc::format!("Edge references non-existent source task {}", edge.from_task),
                ));
            }
            if !self.tasks.contains_key(&edge.to_task) {
                return Err(AdapterError::TranslationError(
                    alloc::format!("Edge references non-existent target task {}", edge.to_task),
                ));
            }
        }

        // Check dependency references are consistent with edges
        for (task_id, task) in &self.tasks {
            for dep_id in &task.depends_on {
                let has_edge = self.edges.iter().any(|e| &e.from_task == dep_id && &e.to_task == task_id);
                if !has_edge && !self.tasks.contains_key(dep_id) {
                    return Err(AdapterError::TranslationError(
                        alloc::format!("Task {} depends on non-existent task {}", task_id, dep_id),
                    ));
                }
            }
        }

        // Verify no cycles via topological sort
        self.topological_sort()?;

        Ok(())
    }

    /// Gets parallelizable groups of tasks (tasks with no dependencies).
    /// Sec 4.2: Parallelization Analysis
    pub fn get_parallelizable_groups(&self) -> Vec<Vec<StepId>> {
        let mut groups = Vec::new();
        let mut processed = BTreeMap::new();

        // Iteratively find groups with no unprocessed dependencies
        loop {
            let mut current_group = Vec::new();

            for (task_id, task) in &self.tasks {
                if processed.contains_key(task_id) {
                    continue;
                }

                let all_deps_processed = task.depends_on.iter().all(|dep| processed.contains_key(dep));
                if all_deps_processed {
                    current_group.push(*task_id);
                }
            }

            if current_group.is_empty() {
                break;
            }

            for task_id in &current_group {
                processed.insert(*task_id, true);
            }

            groups.push(current_group);
        }

        groups
    }
}

impl Default for TaskDag {
    fn default() -> Self {
        Self::new()
    }
}

/// Translator from SK planner output to CT spawn requests.
/// Sec 4.3: SK Planner Translator
pub struct SkPlannerTranslator;

impl SkPlannerTranslator {
    /// Translates a sequence of planner steps to a task DAG with spawn requests.
    /// Sec 4.3: Plan-to-DAG Translation
    pub fn translate_plan_to_dag(steps: &[PlannerStep]) -> AdapterResult<TaskDag> {
        let mut dag = TaskDag::new();

        if steps.is_empty() {
            return Err(AdapterError::TranslationError(
                "Empty plan: no steps to translate".to_string(),
            ));
        }

        // Create spawn request for each step
        for (idx, step) in steps.iter().enumerate() {
            let request_id = alloc::format!("sk-spawn-{}", idx);
            let task_id = idx as StepId;
            
            let mut spawn_request = CtSpawnRequest::new(
                request_id,
                task_id,
                step.function_ref.clone(),
            );

            // Copy parameters
            spawn_request.inputs = step.parameters.clone();

            // Dependencies: previous step in sequence (for sequential execution)
            if idx > 0 {
                spawn_request.depends_on.push((idx - 1) as StepId);
            }

            dag.add_task(spawn_request);
        }

        // Build dependency edges for data flow
        for (idx, step) in steps.iter().enumerate() {
            if let Some(output_var) = &step.output_var {
                // Find steps that use this output variable as input
                for (next_idx, next_step) in steps.iter().enumerate().skip(idx + 1) {
                    if next_step.parameters.values().any(|v| v == output_var) {
                        let edge = DependencyEdge {
                            from_task: idx as StepId,
                            to_task: next_idx as StepId,
                            data_var: output_var.clone(),
                        };
                        dag.add_edge(edge);
                    }
                }
            }
        }

        // Validate DAG before returning
        dag.validate()?;

        Ok(dag)
    }

    /// Extracts spawn requests from a DAG in execution order.
    /// Sec 4.3: Spawn Request Extraction
    pub fn extract_spawn_requests(dag: &TaskDag) -> AdapterResult<Vec<CtSpawnRequest>> {
        let sorted_ids = dag.topological_sort()?;
        let mut requests = Vec::new();

        for task_id in sorted_ids {
            if let Some(request) = dag.tasks.get(&task_id) {
                requests.push(request.clone());
            }
        }

        Ok(requests)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use alloc::vec;

    #[test]
    fn test_planner_step_creation() {
        let step = PlannerStep::new(0, "Plugin.Function".into());
        assert_eq!(step.sequence, 0);
        assert_eq!(step.function_ref, "Plugin.Function");
        assert!(step.output_var.is_none());
    }

    #[test]
    fn test_spawn_request_creation() {
        let request = CtSpawnRequest::new(
            "req-1".into(),
            1,
            "Plugin.Func".into(),
        );
        assert_eq!(request.task_id, 1);
        assert_eq!(request.priority, 50);
        assert_eq!(request.timeout_ms, 30000);
        assert!(request.depends_on.is_empty());
    }

    #[test]
    fn test_dependency_edge_creation() {
        let edge = DependencyEdge {
            from_task: 1,
            to_task: 2,
            data_var: "output".into(),
        };
        assert_eq!(edge.from_task, 1);
        assert_eq!(edge.to_task, 2);
    }

    #[test]
    fn test_task_dag_creation() {
        let dag = TaskDag::new();
        assert!(dag.tasks.is_empty());
        assert!(dag.edges.is_empty());
    }

    #[test]
    fn test_task_dag_add_task() {
        let mut dag = TaskDag::new();
        let request = CtSpawnRequest::new("req-1".into(), 1, "Plugin.Func".into());
        dag.add_task(request);
        assert_eq!(dag.tasks.len(), 1);
    }

    #[test]
    fn test_task_dag_topological_sort() {
        let mut dag = TaskDag::new();
        
        let mut req1 = CtSpawnRequest::new("req-1".into(), 1, "Plugin.Func1".into());
        req1.priority = 50;
        
        let mut req2 = CtSpawnRequest::new("req-2".into(), 2, "Plugin.Func2".into());
        req2.depends_on.push(1);
        
        dag.add_task(req1);
        dag.add_task(req2);

        let sorted = dag.topological_sort();
        assert!(sorted.is_ok());
        let order = sorted.unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], 1);
        assert_eq!(order[1], 2);
    }

    #[test]
    fn test_task_dag_cyclic_dependency_detection() {
        let mut dag = TaskDag::new();
        
        let mut req1 = CtSpawnRequest::new("req-1".into(), 1, "Plugin.Func1".into());
        req1.depends_on.push(2);
        
        let mut req2 = CtSpawnRequest::new("req-2".into(), 2, "Plugin.Func2".into());
        req2.depends_on.push(1);
        
        dag.add_task(req1);
        dag.add_task(req2);

        let result = dag.topological_sort();
        assert!(result.is_err());
    }

    #[test]
    fn test_task_dag_validate() {
        let mut dag = TaskDag::new();
        
        let req1 = CtSpawnRequest::new("req-1".into(), 1, "Plugin.Func1".into());
        dag.add_task(req1);

        // Valid DAG with no edges
        assert!(dag.validate().is_ok());
    }

    #[test]
    fn test_task_dag_validate_invalid_edge() {
        let mut dag = TaskDag::new();
        
        let req1 = CtSpawnRequest::new("req-1".into(), 1, "Plugin.Func1".into());
        dag.add_task(req1);

        // Add edge to non-existent task
        let edge = DependencyEdge {
            from_task: 1,
            to_task: 99,
            data_var: "var".into(),
        };
        dag.add_edge(edge);

        assert!(dag.validate().is_err());
    }

    #[test]
    fn test_parallelizable_groups() {
        let mut dag = TaskDag::new();
        
        // Task 1 and 2 can run in parallel (no dependencies)
        let req1 = CtSpawnRequest::new("req-1".into(), 1, "Plugin.Func1".into());
        let req2 = CtSpawnRequest::new("req-2".into(), 2, "Plugin.Func2".into());
        
        // Task 3 depends on both 1 and 2
        let mut req3 = CtSpawnRequest::new("req-3".into(), 3, "Plugin.Func3".into());
        req3.depends_on.push(1);
        req3.depends_on.push(2);

        dag.add_task(req1);
        dag.add_task(req2);
        dag.add_task(req3);

        let groups = dag.get_parallelizable_groups();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 2); // First group has tasks 1 and 2
        assert_eq!(groups[1].len(), 1); // Second group has task 3
    }

    #[test]
    fn test_planner_translator_simple_sequence() {
        let steps = vec![
            PlannerStep::new(0, "Plugin.Step1".into()),
            PlannerStep::new(1, "Plugin.Step2".into()),
        ];

        let result = SkPlannerTranslator::translate_plan_to_dag(&steps);
        assert!(result.is_ok());
        
        let dag = result.unwrap();
        assert_eq!(dag.tasks.len(), 2);
    }

    #[test]
    fn test_planner_translator_empty_steps_error() {
        let steps = vec![];
        let result = SkPlannerTranslator::translate_plan_to_dag(&steps);
        assert!(result.is_err());
    }

    #[test]
    fn test_planner_translator_with_data_flow() {
        let mut step1 = PlannerStep::new(0, "Plugin.Step1".into());
        step1.output_var = Some("result1".into());

        let mut step2 = PlannerStep::new(1, "Plugin.Step2".into());
        step2.parameters.insert("input".into(), "result1".into());

        let steps = vec![step1, step2];

        let result = SkPlannerTranslator::translate_plan_to_dag(&steps);
        assert!(result.is_ok());
        
        let dag = result.unwrap();
        assert!(!dag.edges.is_empty());
    }

    #[test]
    fn test_extract_spawn_requests() {
        let mut step1 = PlannerStep::new(0, "Plugin.Step1".into());
        let mut step2 = PlannerStep::new(1, "Plugin.Step2".into());
        
        let steps = vec![step1, step2];

        let dag = SkPlannerTranslator::translate_plan_to_dag(&steps).unwrap();
        let requests = SkPlannerTranslator::extract_spawn_requests(&dag);
        
        assert!(requests.is_ok());
        let reqs = requests.unwrap();
        assert_eq!(reqs.len(), 2);
    }
}
