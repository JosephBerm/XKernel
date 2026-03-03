"""Core types for the XKernal Python SDK.

Mirrors the Rust types from daemon/cs-daemon/src/models.rs and
sdk/csci/src/types.rs. Uses dataclasses (not Pydantic) to keep
the dependency footprint minimal.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import NewType

# ── Identity types ───────────────────────────────────────────────────────────

AgentId = NewType("AgentId", str)
ChannelId = NewType("ChannelId", int)
ToolBindingId = NewType("ToolBindingId", str)

# ── Enums (mirror Rust string-encoded enums) ─────────────────────────────────


class RestartPolicy(str, Enum):
    """Agent restart policy — matches daemon CreateAgentRequest.restart_policy."""
    NEVER = "never"
    ON_FAILURE = "on_failure"
    ALWAYS = "always"


class EffectClass(str, Enum):
    """Tool effect classification — matches daemon RegisterToolRequest.effect_class."""
    READ_ONLY = "read_only"
    WRITE_REVERSIBLE = "write_reversible"
    WRITE_IRREVERSIBLE = "write_irreversible"


class FrameworkType(str, Enum):
    """AI framework type — matches daemon CreateAgentRequest.framework."""
    LANGCHAIN = "langchain"
    CREWAI = "crewai"
    AUTOGEN = "autogen"
    SEMANTIC_KERNEL = "semantic_kernel"
    CUSTOM = "custom"


class AgentState(str, Enum):
    """Runtime state of an agent — matches daemon AgentResponse.state."""
    CREATED = "Created"
    STARTING = "Starting"
    RUNNING = "Running"
    STOPPED = "Stopped"
    FAILED = "Failed"
    RESTARTING = "Restarting"


class Capability(str, Enum):
    """Kernel capabilities that can be granted to agents."""
    TASK = "task"
    MEMORY = "memory"
    TOOL = "tool"
    CHANNEL = "channel"
    TELEMETRY = "telemetry"


# ── Configuration dataclasses ────────────────────────────────────────────────


@dataclass(frozen=True)
class AgentConfig:
    """Configuration for registering an agent with the daemon.

    Maps 1:1 to the daemon's CreateAgentRequest JSON body.
    """
    name: str
    framework: FrameworkType = FrameworkType.CUSTOM
    entrypoint: str | None = None
    working_dir: str | None = None
    env: dict[str, str] = field(default_factory=dict)
    priority: int = 128
    restart_policy: RestartPolicy = RestartPolicy.NEVER
    max_retries: int = 3
    capabilities: list[Capability] = field(default_factory=list)

    def to_dict(self) -> dict:
        """Serialize to the JSON shape expected by POST /api/v1/agents."""
        d: dict = {
            "name": self.name,
            "framework": self.framework.value,
            "priority": self.priority,
            "restart_policy": self.restart_policy.value,
            "max_retries": self.max_retries,
            "capabilities": [c.value for c in self.capabilities],
            "env": self.env,
        }
        if self.entrypoint is not None:
            d["entrypoint"] = self.entrypoint
        if self.working_dir is not None:
            d["working_dir"] = self.working_dir
        return d


@dataclass(frozen=True)
class ToolConfig:
    """Configuration for registering a tool with the daemon.

    Maps 1:1 to the daemon's RegisterToolRequest JSON body.
    """
    name: str
    description: str = ""
    input_schema: str = "{}"
    output_schema: str = "{}"
    effect_class: EffectClass = EffectClass.READ_ONLY
    agent_id: str | None = None

    def to_dict(self) -> dict:
        """Serialize to the JSON shape expected by POST /api/v1/tools."""
        d: dict = {
            "name": self.name,
            "description": self.description,
            "input_schema": self.input_schema,
            "output_schema": self.output_schema,
            "effect_class": self.effect_class.value,
        }
        if self.agent_id is not None:
            d["agent_id"] = self.agent_id
        return d


@dataclass(frozen=True)
class ChannelConfig:
    """Configuration for creating an IPC channel.

    Maps 1:1 to the daemon's CreateChannelRequest JSON body.
    """
    sender: str
    receiver: str
    capacity: int = 256

    def to_dict(self) -> dict:
        """Serialize to the JSON shape expected by POST /api/v1/channels."""
        return {
            "sender": self.sender,
            "receiver": self.receiver,
            "capacity": self.capacity,
        }


@dataclass(frozen=True)
class AllocateMemoryConfig:
    """Configuration for a memory allocation request."""
    pages: int
    owner_ct_id: int

    def to_dict(self) -> dict:
        return {"pages": self.pages, "owner_ct_id": self.owner_ct_id}
