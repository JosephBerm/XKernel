"""XKernal Python SDK — build AI agents on the cognitive substrate OS.

Usage::

    import xkernal

    @xkernal.agent(name="my_agent", capabilities=["task", "tool"])
    async def my_agent(ctx):
        ctx.log.info("Hello from XKernal!")

    @xkernal.tool(name="search", effect_class="read_only")
    def search(query: str) -> list[dict]:
        return [{"result": query}]

    my_agent.attach_tool(search)
    xkernal.run(my_agent)
"""

from ._version import __version__
from .agent import Agent, agent
from .client import DaemonClient
from .context import AgentContext, current_context
from .errors import (
    ChannelClosedError,
    CsciError,
    CsciErrorCode,
    DaemonConnectionError,
    ErrorCategory,
    InvalidArgumentError,
    NotFoundError,
    PermissionError_,
    TimeoutError_,
)
from .ipc import Channel
from .logging import get_logger, setup_logging
from .runtime import run
from .tool import Tool, tool
from .types import (
    AgentConfig,
    AgentId,
    AgentState,
    AllocateMemoryConfig,
    Capability,
    ChannelConfig,
    ChannelId,
    EffectClass,
    FrameworkType,
    RestartPolicy,
    ToolBindingId,
    ToolConfig,
)

__all__ = [
    # Version
    "__version__",
    # Core API
    "agent",
    "tool",
    "run",
    # Classes
    "Agent",
    "AgentContext",
    "Channel",
    "DaemonClient",
    "Tool",
    # Context
    "current_context",
    "get_logger",
    "setup_logging",
    # Types
    "AgentConfig",
    "AgentId",
    "AgentState",
    "AllocateMemoryConfig",
    "Capability",
    "ChannelConfig",
    "ChannelId",
    "EffectClass",
    "FrameworkType",
    "RestartPolicy",
    "ToolBindingId",
    "ToolConfig",
    # Errors
    "ChannelClosedError",
    "CsciError",
    "CsciErrorCode",
    "DaemonConnectionError",
    "ErrorCategory",
    "InvalidArgumentError",
    "NotFoundError",
    "PermissionError_",
    "TimeoutError_",
]
