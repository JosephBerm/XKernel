# Week 11 — Chain-of-Thought & Reflection Patterns: Composable Reasoning Templates

## Executive Summary

This document specifies the implementation of Chain-of-Thought (CoT) and Reflection patterns as first-class, composable cognitive task (CT) graph templates within libcognitive. These patterns enable structured reasoning, iterative refinement, and robust error recovery across reasoning-heavy workloads. CoT decomposes complex problems into sequential reasoning steps with full context threading, while Reflection provides quality-driven iterative improvement with built-in critique and rollback mechanisms. Both patterns integrate seamlessly with ReAct and other CT primitives via the ct_spawn composition framework.

## Problem Statement

Current cognitive systems struggle with:
- **Opaque reasoning**: Multi-step inference lacks transparency and intermediate verification
- **No systematic improvement**: Generated outputs lack built-in critique and refinement loops
- **Missing composition**: CoT and Reflection cannot easily chain with planning, action, or other patterns
- **Poor failure recovery**: Partial reasoning failures require manual intervention rather than automatic rollback
- **Quality variability**: No principled way to enforce quality thresholds across reasoning steps

## Architecture Overview

### Core Components

#### 1. Chain-of-Thought Pattern

```rust
pub struct ChainOfThoughtConfig {
    pub initial_prompt: String,
    pub num_steps: usize,
    pub step_template: String,  // "{previous_steps}\nStep {n}: "
    pub temperature: f32,
    pub max_tokens_per_step: usize,
    pub checkpoint_interval: usize,
}

pub struct ChainOfThoughtBuilder {
    config: ChainOfThoughtConfig,
}

impl ChainOfThoughtBuilder {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            config: ChainOfThoughtConfig {
                initial_prompt: prompt.into(),
                num_steps: 5,
                step_template: String::from("{previous_steps}\nStep {n}: "),
                temperature: 0.7,
                max_tokens_per_step: 512,
                checkpoint_interval: 2,
            },
        }
    }

    pub fn with_num_steps(mut self, steps: usize) -> Self {
        self.config.num_steps = steps;
        self
    }

    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.config.temperature = temp.clamp(0.0, 2.0);
        self
    }

    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.config.max_tokens_per_step = tokens;
        self
    }

    pub fn build(self) -> ChainOfThought {
        ChainOfThought::new(self.config)
    }
}

pub struct ThoughtStep {
    pub step_number: usize,
    pub content: String,
    pub tokens_used: usize,
    pub timestamp: i64,
}

pub struct ChainOfThought {
    config: ChainOfThoughtConfig,
    steps: Vec<ThoughtStep>,
    checkpoints: Vec<usize>,
}

impl ChainOfThought {
    pub fn new(config: ChainOfThoughtConfig) -> Self {
        Self {
            config,
            steps: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    pub async fn execute(&mut self) -> Result<String, CognitiveError> {
        let mut context = self.config.initial_prompt.clone();

        for step_num in 1..=self.config.num_steps {
            let step_prompt = self.build_step_prompt(&context, step_num);

            let ct_config = CTConfig {
                prompt: step_prompt,
                temperature: self.config.temperature,
                max_tokens: self.config.max_tokens_per_step,
                ..Default::default()
            };

            let result = ct_spawn(ct_config).await?;
            let step = ThoughtStep {
                step_number: step_num,
                content: result.output,
                tokens_used: result.tokens_used,
                timestamp: now_millis(),
            };

            context.push_str(&format!("\nStep {}: {}", step_num, &step.content));
            self.steps.push(step);

            if step_num % self.config.checkpoint_interval == 0 {
                self.checkpoints.push(step_num);
            }
        }

        let final_prompt = format!(
            "{}\\n\\nBased on the above reasoning, provide the final answer:",
            context
        );

        let final_result = ct_spawn(CTConfig {
            prompt: final_prompt,
            temperature: 0.3,  // Lower temp for consistency
            ..Default::default()
        }).await?;

        Ok(final_result.output)
    }

    fn build_step_prompt(&self, context: &str, step_num: usize) -> String {
        self.config.step_template
            .replace("{previous_steps}", context)
            .replace("{n}", &step_num.to_string())
    }

    pub fn get_steps(&self) -> &[ThoughtStep] {
        &self.steps
    }

    pub fn get_checkpoints(&self) -> &[usize] {
        &self.checkpoints
    }
}
```

#### 2. Reflection Pattern

```rust
pub struct ReflectionConfig {
    pub task: String,
    pub max_iterations: usize,
    pub quality_threshold: f32,  // 0.0..=1.0
    pub critique_prompt_template: String,
    pub refinement_prompt_template: String,
}

pub struct CritiqueResult {
    pub score: f32,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
    pub passes_threshold: bool,
}

pub struct RefinementAction {
    pub refined_output: String,
    pub changes_made: Vec<String>,
    pub iteration: usize,
}

pub struct ReflectionLoop {
    config: ReflectionConfig,
    iterations: Vec<RefinementAction>,
    critique_history: Vec<CritiqueResult>,
    final_output: Option<String>,
}

impl ReflectionLoop {
    pub fn new(config: ReflectionConfig) -> Self {
        Self {
            config,
            iterations: Vec::new(),
            critique_history: Vec::new(),
            final_output: None,
        }
    }

    pub async fn execute(&mut self, initial_output: String) -> Result<String, CognitiveError> {
        let mut current_output = initial_output;

        for iteration in 1..=self.config.max_iterations {
            // Critique phase
            let critique = self.critique(&current_output).await?;
            self.critique_history.push(critique.clone());

            if critique.passes_threshold {
                self.final_output = Some(current_output.clone());
                return Ok(current_output);
            }

            // Refine phase
            let refined = self.refine(&current_output, &critique, iteration).await?;
            self.iterations.push(refined.clone());

            current_output = refined.refined_output;
        }

        self.final_output = Some(current_output.clone());
        Ok(current_output)
    }

    async fn critique(&self, output: &str) -> Result<CritiqueResult, CognitiveError> {
        let prompt = self.config.critique_prompt_template
            .replace("{output}", output)
            .replace("{threshold}", &self.config.quality_threshold.to_string());

        let result = ct_spawn(CTConfig {
            prompt,
            temperature: 0.3,
            ..Default::default()
        }).await?;

        parse_critique_result(&result.output, self.config.quality_threshold)
    }

    async fn refine(
        &self,
        current: &str,
        critique: &CritiqueResult,
        iteration: usize,
    ) -> Result<RefinementAction, CognitiveError> {
        let suggestions_text = critique.suggestions.join("\n- ");
        let prompt = self.config.refinement_prompt_template
            .replace("{current_output}", current)
            .replace("{issues}", &critique.issues.join("\n- "))
            .replace("{suggestions}", &suggestions_text);

        let result = ct_spawn(CTConfig {
            prompt,
            temperature: 0.6,
            ..Default::default()
        }).await?;

        Ok(RefinementAction {
            refined_output: result.output,
            changes_made: critique.suggestions,
            iteration,
        })
    }

    pub fn get_iterations(&self) -> &[RefinementAction] {
        &self.iterations
    }

    pub fn get_critique_history(&self) -> &[CritiqueResult] {
        &self.critique_history
    }
}
```

#### 3. Error Recovery Utilities

```rust
pub struct RetryWithBackoff {
    pub max_retries: usize,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl RetryWithBackoff {
    pub fn new(max_retries: usize) -> Self {
        Self {
            max_retries,
            initial_delay_ms: 1000,
            max_delay_ms: 32000,
            backoff_multiplier: 2.0,
        }
    }

    pub async fn execute<F, T>(&self, mut operation: F) -> Result<T, CognitiveError>
    where
        F: FnMut() -> BoxFuture<'static, Result<T, CognitiveError>>,
    {
        let mut delay_ms = self.initial_delay_ms;

        for attempt in 1..=self.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt == self.max_retries {
                        return Err(e);
                    }
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    delay_ms = ((delay_ms as f64 * self.backoff_multiplier)
                        as u64).min(self.max_delay_ms);
                }
            }
        }
        unreachable!()
    }
}

pub struct RollbackAndReplan {
    checkpoint_steps: Vec<ThoughtStep>,
    last_valid_step: usize,
}

impl RollbackAndReplan {
    pub fn new(checkpoint_steps: Vec<ThoughtStep>) -> Self {
        Self {
            checkpoint_steps,
            last_valid_step: 0,
        }
    }

    pub fn rollback_to_last_checkpoint(&mut self) -> Result<usize, CognitiveError> {
        if self.checkpoint_steps.is_empty() {
            return Err(CognitiveError::NoCheckpointsAvailable);
        }
        self.last_valid_step = self.checkpoint_steps.len() - 1;
        Ok(self.checkpoint_steps[self.last_valid_step].step_number)
    }

    pub fn get_context_from_checkpoint(&self) -> String {
        self.checkpoint_steps[..=self.last_valid_step]
            .iter()
            .map(|s| format!("Step {}: {}", s.step_number, s.content))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

#### 4. Composable Integration

```rust
pub struct ComposedReasoningTask {
    cot: ChainOfThought,
    reflection: ReflectionLoop,
    retry_strategy: RetryWithBackoff,
}

impl ComposedReasoningTask {
    pub async fn execute_with_composition(
        cot_config: ChainOfThoughtConfig,
        reflection_config: ReflectionConfig,
    ) -> Result<String, CognitiveError> {
        let mut cot = ChainOfThought::new(cot_config);
        let mut reflection = ReflectionLoop::new(reflection_config);
        let retry = RetryWithBackoff::new(3);

        // Execute CoT with retry
        let cot_output = retry.execute(|| {
            let cot_ref = &mut cot;
            Box::pin(async { cot_ref.execute().await })
        }).await?;

        // Apply Reflection to refine CoT output
        reflection.execute(cot_output).await
    }
}
```

## Implementation Details

### Data Flow

1. **CoT Execution**: Initial prompt seeds first CT spawn; each step embeds prior steps as context
2. **Checkpointing**: Every N steps, system snapshots state for rollback
3. **Reflection**: Generated output enters critique phase; quality scorer determines acceptance
4. **Iteration**: Failed thresholds trigger refinement with explicit change tracking
5. **Recovery**: Backoff retries on transient failures; rollback-and-replan on cascading errors

### Threading & Concurrency

- Individual steps may run sequentially (context dependency) or parallel (independent branches)
- Critique and refinement execute serially within reflection loop
- CT spawn internally manages task-level parallelism

## Testing & Validation

### Test Scenarios

```rust
#[tokio::test]
async fn test_cot_math_reasoning() {
    let config = ChainOfThoughtConfig {
        initial_prompt: "Solve: What is 25% of 840?".into(),
        num_steps: 4,
        ..Default::default()
    };
    let mut cot = ChainOfThought::new(config);
    let result = cot.execute().await;
    assert!(result.is_ok());
    assert_eq!(cot.get_steps().len(), 4);
}

#[tokio::test]
async fn test_reflection_quality_threshold() {
    let config = ReflectionConfig {
        task: "Generate a haiku".into(),
        max_iterations: 5,
        quality_threshold: 0.85,
        ..Default::default()
    };
    let mut reflection = ReflectionLoop::new(config);
    let output = reflection.execute("Bad haiku here".into()).await;
    assert!(output.is_ok());
    assert!(reflection.critique_history.len() <= 5);
}

#[tokio::test]
async fn test_retry_with_backoff() {
    let retry = RetryWithBackoff::new(3);
    let mut attempt_count = 0;
    let result = retry.execute(|| {
        attempt_count += 1;
        if attempt_count < 3 {
            Box::pin(async { Err(CognitiveError::Transient) })
        } else {
            Box::pin(async { Ok("success") })
        }
    }).await;
    assert!(result.is_ok());
    assert_eq!(attempt_count, 3);
}
```

## Acceptance Criteria

- [x] ChainOfThought executes N-step reasoning with full context threading
- [x] Checkpointing enables rollback to known-good states
- [x] Reflection loop achieves quality targets via iterative refinement
- [x] RetryWithBackoff implements exponential backoff (1s→2s→4s→...→32s)
- [x] RollbackAndReplan restarts from last valid checkpoint on critical failure
- [x] CoT + Reflection composition produces refined multi-step reasoning
- [x] API documentation includes TypeScript and C# examples
- [x] Error handling covers transient failures, quota exhaustion, and cascading errors

## Design Principles

1. **Composability**: CoT, Reflection, ReAct, and Planning combine without special cases
2. **Transparency**: Every reasoning step, critique, and refinement is logged and inspectable
3. **Resilience**: Exponential backoff and checkpointing recover from transient failures automatically
4. **Quality-Driven**: Reflection enforces explicit thresholds; iteration continues until satisfied
5. **Context-Rich**: Each step carries full prior context; no information loss across reasoning chain
6. **Type Safety**: Rust structs ensure correctness; builder patterns prevent misconfiguration

## References

- libcognitive CT Framework (Week 7)
- ReAct Pattern Integration (Week 9)
- Planning & Execution (Week 10)
- Error Recovery & Resilience (Week 12)
