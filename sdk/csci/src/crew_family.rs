// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! # CSCI Crew Family Syscalls
//!
//! Crew family syscalls manage agent crews and collective coordination:
//! - **crew_create**: Create a new AgentCrew
//! - **crew_join**: Join an existing crew
//! - **crew_leave**: Leave a crew
//! - **crew_query**: Query crew status
//!
//! # Engineering Plan Reference
//! Section 7.5: Crew Family Specification.

use crate::error_codes::CsciErrorCode;
use crate::syscall::{ParamType, ReturnType, SyscallDefinition, SyscallFamily, SyscallParam};
use crate::types::{AgentID, CapabilitySet, CTID};


use core::fmt;

/// Crew family syscall numbers.
pub mod number {
    /// crew_create syscall number within Crew family.
    pub const CREW_CREATE: u8 = 0;
    /// crew_join syscall number within Crew family.
    pub const CREW_JOIN: u8 = 1;
    /// crew_leave syscall number within Crew family.
    pub const CREW_LEAVE: u8 = 2;
    /// crew_query syscall number within Crew family.
    pub const CREW_QUERY: u8 = 3;
}

/// Identifier for an AgentCrew.
///
/// A globally unique identifier assigned at crew creation.
/// References a specific crew collective.
///
/// # Engineering Plan Reference
/// Section 7.5.1: Crew identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CrewID(pub u64);

impl fmt::Display for CrewID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CREW-{:x}", self.0)
    }
}

/// Role within a crew.
///
/// Defines the responsibility level of an agent within a crew collective.
///
/// # Engineering Plan Reference
/// Section 7.5.2: Crew roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CrewRole {
    /// Coordinator: manages crew planning and task distribution.
    Coordinator = 0,
    /// Worker: executes assigned tasks within the crew's mission.
    Worker = 1,
    /// Specialist: provides specialized expertise for particular problem domains.
    Specialist = 2,
}

impl fmt::Display for CrewRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coordinator => write!(f, "Coordinator"),
            Self::Worker => write!(f, "Worker"),
            Self::Specialist => write!(f, "Specialist"),
        }
    }
}

impl CrewRole {
    /// Convert a u8 to a CrewRole.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Coordinator),
            1 => Some(Self::Worker),
            2 => Some(Self::Specialist),
            _ => None,
        }
    }
}

/// Configuration for creating a new crew.
///
/// Specifies parameters for crew creation including mission description,
/// initial coordinator, initial members, and budget settings.
///
/// # Engineering Plan Reference
/// Section 7.5.3: Crew creation configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewConfig {
    /// Mission description (max 256 bytes).
    pub mission: [u8; 256],
    /// Mission description length.
    pub mission_len: usize,
    /// Agent ID of the coordinator.
    pub coordinator_agent: AgentID,
    /// Initial crew members (agent IDs).
    pub initial_members: [u64; 32],
    /// Number of initial members.
    pub members_count: usize,
    /// Total collective budget for the crew.
    pub collective_budget: u64,
    /// Scheduling affinity hint (0-255, higher = tighter coupling).
    pub scheduling_affinity: u8,
}

impl CrewConfig {
    /// Create a new crew configuration with defaults.
    pub fn new() -> Self {
        Self {
            mission: [0; 256],
            mission_len: 0,
            coordinator_agent: AgentID(0),
            initial_members: [0; 32],
            members_count: 0,
            collective_budget: 0,
            scheduling_affinity: 128,
        }
    }

    /// Set the mission description.
    pub fn with_mission(mut self, mission: &[u8]) -> Self {
        let len = core::cmp::min(mission.len(), 256);
        self.mission[..len].copy_from_slice(&mission[..len]);
        self.mission_len = len;
        self
    }

    /// Set the coordinator agent.
    pub fn with_coordinator(mut self, coordinator: AgentID) -> Self {
        self.coordinator_agent = coordinator;
        self
    }

    /// Set the collective budget.
    pub fn with_budget(mut self, budget: u64) -> Self {
        self.collective_budget = budget;
        self
    }

    /// Get the mission as a string slice (if valid UTF-8).
    pub fn mission_str(&self) -> Option<&str> {
        if self.mission_len == 0 {
            return Some("");
        }
        core::str::from_utf8(&self.mission[..self.mission_len]).ok()
    }
}

impl Default for CrewConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Status information for a crew.
///
/// Returned by crew_query syscall. Contains information about
/// crew membership, health, and resource utilization.
///
/// # Engineering Plan Reference
/// Section 7.5.5: Crew status information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewStatus {
    /// Crew ID.
    pub crew_id: CrewID,
    /// Number of active members.
    pub members: usize,
    /// Coordinator agent ID.
    pub coordinator: AgentID,
    /// Number of active tasks being executed.
    pub active_tasks: u64,
    /// Remaining collective budget.
    pub budget_remaining: u64,
    /// Crew health indicator (0-100, 100 = optimal).
    pub health: u8,
}

impl fmt::Display for CrewStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CrewStatus {{ id: {}, members: {}, coordinator: {}, active_tasks: {}, budget: {}, health: {}% }}",
            self.crew_id, self.members, self.coordinator, self.active_tasks, self.budget_remaining, self.health
        )
    }
}

/// Get the definition of the crew_create syscall.
///
/// **crew_create**: Create a new AgentCrew.
///
/// Creates a new crew with a mission, coordinator, initial members, and budget.
/// The crew is ready for task assignment immediately after creation.
///
/// # Parameters
/// - `config`: (CrewConfig) Crew configuration including mission, coordinator, members, budget
///
/// # Returns
/// - Success: CrewID of the newly created crew
/// - Error: CS_EINVAL (invalid configuration), CS_ENOMEM (insufficient memory),
///          CS_EPERM (caller lacks Crew capability), CS_ENOENT (coordinator not found)
///
/// # Preconditions
/// - Caller must have Crew family capability (CAP_CREW_FAMILY)
/// - `coordinator_agent` must be a valid, existing agent
/// - `collective_budget` must be > 0
/// - `initial_members` count must be <= 32
///
/// # Postconditions
/// - Crew is created with specified configuration
/// - Crew assigned immutable CrewID
/// - All initial members added with Worker role
/// - Coordinator assigned Coordinator role
///
/// # Engineering Plan Reference
/// Section 7.5.1: crew_create specification.
pub fn crew_create_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "crew_create",
        SyscallFamily::Crew,
        number::CREW_CREATE,
        ReturnType::Identifier,
        CapabilitySet::CAP_CREW_FAMILY,
        "Create a new AgentCrew with mission and members",
    )
    .with_param(SyscallParam::new(
        "config",
        ParamType::Config,
        "Crew configuration (mission, coordinator, members, budget, affinity)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEinval)
    .with_error(CsciErrorCode::CsEnomem)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEnoent)
    .with_preconditions(
        "Caller has Crew capability; coordinator is valid; budget > 0; members count <= 32",
    )
    .with_postconditions(
        "Crew created with specified config; initial members added; coordinator assigned",
    )
}

/// Get the definition of the crew_join syscall.
///
/// **crew_join**: Agent joins an existing crew.
///
/// Allows an agent to join a crew with a specified role. The joining agent
/// contributes its resources to the crew's budget.
///
/// # Parameters
/// - `crew_id`: (CrewID) ID of the crew to join
/// - `agent_id`: (AgentID) Agent joining the crew
/// - `role`: (CrewRole) Role within the crew (Coordinator, Worker, Specialist)
///
/// # Returns
/// - Success: Unit (operation successful)
/// - Error: CS_ENOENT (crew not found), CS_EPERM (not authorized),
///          CS_EFULL (crew at max capacity)
///
/// # Preconditions
/// - Crew must exist
/// - Agent must be valid and not already a crew member
/// - Caller must have permission to add agents to crew
/// - Crew must not be at max capacity (32 members)
///
/// # Postconditions
/// - Agent added to crew with specified role
/// - Agent's resources integrated into crew budget
/// - Agent now participates in crew coordination
///
/// # Engineering Plan Reference
/// Section 7.5.2: crew_join specification.
pub fn crew_join_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "crew_join",
        SyscallFamily::Crew,
        number::CREW_JOIN,
        ReturnType::Unit,
        CapabilitySet::CAP_CREW_FAMILY,
        "Add an agent to an existing crew",
    )
    .with_param(SyscallParam::new(
        "crew_id",
        ParamType::Identifier,
        "ID of the crew to join",
        false,
    ))
    .with_param(SyscallParam::new(
        "agent_id",
        ParamType::Identifier,
        "Agent joining the crew",
        false,
    ))
    .with_param(SyscallParam::new(
        "role",
        ParamType::Enum,
        "Role within the crew (Coordinator, Worker, Specialist)",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEperm)
    .with_error(CsciErrorCode::CsEexist)
    .with_preconditions(
        "Crew exists; agent is valid and not already member; crew not at capacity",
    )
    .with_postconditions("Agent added to crew; resources integrated; role assigned")
}

/// Get the definition of the crew_leave syscall.
///
/// **crew_leave**: Agent leaves a crew.
///
/// Allows an agent to leave a crew and reclaim its contributed resources.
/// The agent's tasks are reassigned to other crew members if needed.
///
/// # Parameters
/// - `crew_id`: (CrewID) ID of the crew to leave
/// - `agent_id`: (AgentID) Agent leaving the crew
///
/// # Returns
/// - Success: Unit (operation successful)
/// - Error: CS_ENOENT (crew or agent not found), CS_EPERM (not authorized)
///
/// # Preconditions
/// - Crew must exist
/// - Agent must be a member of the crew
/// - Caller must have permission to remove agents from crew
///
/// # Postconditions
/// - Agent removed from crew
/// - Agent's tasks reassigned
/// - Agent's resources reclaimed
///
/// # Engineering Plan Reference
/// Section 7.5.3: crew_leave specification.
pub fn crew_leave_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "crew_leave",
        SyscallFamily::Crew,
        number::CREW_LEAVE,
        ReturnType::Unit,
        CapabilitySet::CAP_CREW_FAMILY,
        "Remove an agent from a crew",
    )
    .with_param(SyscallParam::new(
        "crew_id",
        ParamType::Identifier,
        "ID of the crew to leave",
        false,
    ))
    .with_param(SyscallParam::new(
        "agent_id",
        ParamType::Identifier,
        "Agent leaving the crew",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEnoent)
    .with_error(CsciErrorCode::CsEperm)
    .with_preconditions("Crew exists; agent is member of crew; caller authorized")
    .with_postconditions("Agent removed; tasks reassigned; resources reclaimed")
}

/// Get the definition of the crew_query syscall.
///
/// **crew_query**: Query crew status and information.
///
/// Returns detailed status information about a crew including membership,
/// active tasks, resource utilization, and health metrics.
///
/// # Parameters
/// - `crew_id`: (CrewID) ID of the crew to query
///
/// # Returns
/// - Success: CrewStatus containing crew information
/// - Error: CS_ENOENT (crew not found)
///
/// # Preconditions
/// - Crew must exist
/// - Caller may need Crew capability depending on crew permissions
///
/// # Postconditions
/// - Current crew status returned
/// - No crew state changes
///
/// # Engineering Plan Reference
/// Section 7.5.4: crew_query specification.
pub fn crew_query_definition() -> SyscallDefinition {
    SyscallDefinition::new(
        "crew_query",
        SyscallFamily::Crew,
        number::CREW_QUERY,
        ReturnType::Memory,
        CapabilitySet::CAP_CREW_FAMILY,
        "Query status of a crew",
    )
    .with_param(SyscallParam::new(
        "crew_id",
        ParamType::Identifier,
        "ID of the crew to query",
        false,
    ))
    .with_error(CsciErrorCode::CsSuccess)
    .with_error(CsciErrorCode::CsEnoent)
    .with_preconditions("Crew exists")
    .with_postconditions("Current crew status returned; no state changes")
}

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn test_crew_id_display() {
        let crew_id = CrewID(0x1234567890abcdef);
        assert_eq!(crew_id.to_string(), "CREW-1234567890abcdef");
    }

    #[test]
    fn test_crew_role_display() {
        assert_eq!(CrewRole::Coordinator.to_string(), "Coordinator");
        assert_eq!(CrewRole::Worker.to_string(), "Worker");
        assert_eq!(CrewRole::Specialist.to_string(), "Specialist");
    }

    #[test]
    fn test_crew_role_from_u8() {
        assert_eq!(CrewRole::from_u8(0), Some(CrewRole::Coordinator));
        assert_eq!(CrewRole::from_u8(1), Some(CrewRole::Worker));
        assert_eq!(CrewRole::from_u8(2), Some(CrewRole::Specialist));
        assert_eq!(CrewRole::from_u8(99), None);
    }

    #[test]
    fn test_crew_config_creation() {
        let config = CrewConfig::new();
        assert_eq!(config.mission_len, 0);
        assert_eq!(config.members_count, 0);
        assert_eq!(config.scheduling_affinity, 128);
    }

    #[test]
    fn test_crew_config_with_mission() {
        let mission = b"Test mission";
        let config = CrewConfig::new().with_mission(mission);
        assert_eq!(config.mission_len, 12);
        assert_eq!(config.mission_str(), Some("Test mission"));
    }

    #[test]
    fn test_crew_config_mission_max_length() {
        let mission = vec![b'a'; 300];
        let config = CrewConfig::new().with_mission(&mission);
        assert_eq!(config.mission_len, 256);
    }

    #[test]
    fn test_crew_config_with_coordinator() {
        let coordinator = AgentID(42);
        let config = CrewConfig::new().with_coordinator(coordinator);
        assert_eq!(config.coordinator_agent, coordinator);
    }

    #[test]
    fn test_crew_config_with_budget() {
        let config = CrewConfig::new().with_budget(1000);
        assert_eq!(config.collective_budget, 1000);
    }

    #[test]
    fn test_crew_config_builder_chain() {
        let coordinator = AgentID(42);
        let config = CrewConfig::new()
            .with_mission(b"Test mission")
            .with_coordinator(coordinator)
            .with_budget(5000);
        assert_eq!(config.mission_str(), Some("Test mission"));
        assert_eq!(config.coordinator_agent, coordinator);
        assert_eq!(config.collective_budget, 5000);
    }

    #[test]
    fn test_crew_status_display() {
        let status = CrewStatus {
            crew_id: CrewID(1),
            members: 5,
            coordinator: AgentID(42),
            active_tasks: 10,
            budget_remaining: 8000,
            health: 95,
        };
        let display_str = status.to_string();
        assert!(display_str.contains("CREW-1"));
        assert!(display_str.contains("5"));
        assert!(display_str.contains("95%"));
    }

    #[test]
    fn test_crew_create_definition() {
        let def = crew_create_definition();
        assert_eq!(def.name, "crew_create");
        assert_eq!(def.family, SyscallFamily::Crew);
        assert_eq!(def.number, number::CREW_CREATE);
        assert!(!def.description.is_empty());
        assert!(!def.error_codes.is_empty());
    }

    #[test]
    fn test_crew_join_definition() {
        let def = crew_join_definition();
        assert_eq!(def.name, "crew_join");
        assert_eq!(def.family, SyscallFamily::Crew);
        assert_eq!(def.number, number::CREW_JOIN);
        assert_eq!(def.parameters.len(), 3);
    }

    #[test]
    fn test_crew_leave_definition() {
        let def = crew_leave_definition();
        assert_eq!(def.name, "crew_leave");
        assert_eq!(def.family, SyscallFamily::Crew);
        assert_eq!(def.number, number::CREW_LEAVE);
        assert_eq!(def.parameters.len(), 2);
    }

    #[test]
    fn test_crew_query_definition() {
        let def = crew_query_definition();
        assert_eq!(def.name, "crew_query");
        assert_eq!(def.family, SyscallFamily::Crew);
        assert_eq!(def.number, number::CREW_QUERY);
        assert_eq!(def.return_type, ReturnType::Memory);
    }
}
