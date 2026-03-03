"""@agent decorator — declares an AI agent for XKernal.

Follows the Prefect pattern: decoration captures metadata, NO daemon
contact occurs until xkernal.run() is called (lazy registration).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable

from .types import AgentConfig, Capability, FrameworkType, RestartPolicy


@dataclass
class Agent:
    """An agent definition created by the @agent decorator.

    This is a wrapper around the user's function plus the metadata
    needed to register with the daemon.
    """
    fn: Callable[..., Any]
    config: AgentConfig
    tools: list[Any] = field(default_factory=list)

    @property
    def name(self) -> str:
        return self.config.name

    def attach_tool(self, tool: Any) -> None:
        """Attach a @tool-decorated function to this agent."""
        self.tools.append(tool)

    async def __call__(self, *args: Any, **kwargs: Any) -> Any:
        from ._sync import ensure_async
        fn = ensure_async(self.fn)
        return await fn(*args, **kwargs)


def agent(
    name: str | None = None,
    *,
    framework: FrameworkType | str = FrameworkType.CUSTOM,
    capabilities: list[Capability | str] | None = None,
    priority: int = 128,
    restart_policy: RestartPolicy | str = RestartPolicy.NEVER,
    max_retries: int = 3,
    tools: list[Any] | None = None,
) -> Callable[[Callable], Agent]:
    """Decorator that declares an AI agent.

    Usage::

        @agent(name="researcher", capabilities=["task", "tool", "channel"])
        async def researcher(ctx):
            result = await ctx.invoke_tool("search", {"query": "AI safety"})
            await ctx.send("writer", result)

    The decorated function receives an ``AgentContext`` as its first argument
    when invoked by ``xkernal.run()``.

    No daemon contact occurs at decoration time. Registration is deferred
    until ``xkernal.run()`` is called.
    """

    def decorator(fn: Callable) -> Agent:
        agent_name = name or fn.__name__

        # Normalize str enum values
        fw = FrameworkType(framework) if isinstance(framework, str) else framework
        rp = RestartPolicy(restart_policy) if isinstance(restart_policy, str) else restart_policy
        caps = [
            Capability(c) if isinstance(c, str) else c
            for c in (capabilities or [])
        ]

        config = AgentConfig(
            name=agent_name,
            framework=fw,
            priority=priority,
            restart_policy=rp,
            max_retries=max_retries,
            capabilities=caps,
        )

        a = Agent(fn=fn, config=config)
        if tools:
            for t in tools:
                a.attach_tool(t)
        return a

    return decorator
