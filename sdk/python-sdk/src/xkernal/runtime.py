"""xkernal.run() — the runtime orchestrator.

Lifecycle:
  1. Connect to daemon, verify health
  2. Register tools with daemon
  3. Register agents with daemon
  4. Create AgentContext, set ContextVar
  5. Run agent functions as asyncio tasks
  6. Heartbeat loop (poll daemon every 10s)
  7. On completion or SIGINT → cleanup
"""

from __future__ import annotations

import asyncio
import signal
import sys
from typing import Any

from .agent import Agent
from .client import DaemonClient
from .context import AgentContext, _current_context
from .logging import get_logger, setup_logging
from .tool import Tool

logger = get_logger()


async def _run_agents(
    *agents: Agent,
    daemon_url: str | None = None,
    heartbeat_interval: float = 10.0,
) -> dict[str, Any]:
    """Core async implementation of xkernal.run()."""

    results: dict[str, Any] = {}
    registered_agent_ids: list[str] = []
    registered_tool_ids: list[str] = []
    shutdown_event = asyncio.Event()

    async with DaemonClient(base_url=daemon_url) as client:
        # 1. Health check
        try:
            health = await client.health()
            logger.info(f"Connected to cs-daemon: {health.get('status', 'unknown')}")
        except Exception as e:
            logger.error(f"Cannot connect to cs-daemon: {e}")
            raise

        # 2. Register all tools from all agents
        for ag in agents:
            for t in ag.tools:
                if isinstance(t, Tool):
                    try:
                        resp = await client.register_tool(t.config)
                        tool_id = resp.get("binding_id", "")
                        registered_tool_ids.append(tool_id)
                        logger.info(f"Registered tool: {t.name} → {tool_id}")
                    except Exception as e:
                        logger.warning(f"Failed to register tool {t.name}: {e}")

        # 3. Register agents and create contexts
        agent_tasks: list[asyncio.Task] = []
        agent_contexts: dict[str, AgentContext] = {}

        for ag in agents:
            try:
                resp = await client.create_agent(ag.config)
                agent_id = resp["id"]
                registered_agent_ids.append(agent_id)
                logger.info(f"Registered agent: {ag.name} → {agent_id}")

                # 4. Create context
                setup_logging(agent_id=agent_id)
                ctx = AgentContext(
                    agent_id=agent_id,
                    agent_name=ag.name,
                    client=client,
                )
                agent_contexts[agent_id] = ctx

                # 5. Spawn agent task
                task = asyncio.create_task(
                    _run_single_agent(ag, ctx, results),
                    name=f"agent-{ag.name}",
                )
                agent_tasks.append(task)

            except Exception as e:
                logger.error(f"Failed to register agent {ag.name}: {e}")
                raise

        # Setup signal handlers for graceful shutdown
        def _signal_handler() -> None:
            logger.info("Received shutdown signal")
            shutdown_event.set()

        loop = asyncio.get_running_loop()
        for sig in (signal.SIGINT, signal.SIGTERM):
            try:
                loop.add_signal_handler(sig, _signal_handler)
            except (NotImplementedError, OSError):
                # Windows doesn't support add_signal_handler for all signals
                pass

        # 6. Heartbeat + wait for completion
        heartbeat_task = asyncio.create_task(
            _heartbeat_loop(client, registered_agent_ids, heartbeat_interval, shutdown_event),
            name="heartbeat",
        )

        try:
            # Wait for all agents to complete or shutdown signal
            done, pending = await asyncio.wait(
                agent_tasks,
                return_when=asyncio.FIRST_EXCEPTION,
            )

            # Check for exceptions
            for task in done:
                if task.exception():
                    logger.error(f"Agent task failed: {task.exception()}")

        except asyncio.CancelledError:
            logger.info("Run cancelled")
        finally:
            # 7. Cleanup
            shutdown_event.set()
            heartbeat_task.cancel()

            # Cancel remaining agent tasks
            for task in agent_tasks:
                if not task.done():
                    task.cancel()

            # Wait for cancellation to complete
            await asyncio.gather(*agent_tasks, heartbeat_task, return_exceptions=True)

            # Cleanup daemon registrations
            for tool_id in registered_tool_ids:
                try:
                    await client.unregister_tool(tool_id)
                except Exception:
                    pass

            for agent_id in registered_agent_ids:
                try:
                    await client.signal_agent(agent_id, "stop", "SDK shutdown")
                except Exception:
                    pass
                try:
                    await client.delete_agent(agent_id)
                except Exception:
                    pass

            logger.info("Shutdown complete")

    return results


async def _run_single_agent(
    ag: Agent,
    ctx: AgentContext,
    results: dict[str, Any],
) -> None:
    """Run a single agent function with its context."""
    token = _current_context.set(ctx)
    try:
        result = await ag(ctx)
        results[ag.name] = result
        logger.info(f"Agent {ag.name} completed")
    except Exception as e:
        logger.error(f"Agent {ag.name} failed: {e}")
        results[ag.name] = e
        raise
    finally:
        _current_context.reset(token)


async def _heartbeat_loop(
    client: DaemonClient,
    agent_ids: list[str],
    interval: float,
    shutdown: asyncio.Event,
) -> None:
    """Poll daemon for agent status every N seconds."""
    while not shutdown.is_set():
        try:
            await asyncio.wait_for(shutdown.wait(), timeout=interval)
            break
        except asyncio.TimeoutError:
            pass

        for agent_id in agent_ids:
            try:
                status = await client.get_agent(agent_id)
                state = status.get("state", "unknown")
                if state in ("Failed", "Stopped"):
                    logger.warning(f"Agent {agent_id} is {state}")
            except Exception:
                pass


def run(
    *agents: Agent,
    daemon_url: str | None = None,
    heartbeat_interval: float = 10.0,
) -> dict[str, Any]:
    """Run one or more agents on the XKernal cognitive substrate.

    This is the main entry point for the SDK. It:
      1. Connects to the cs-daemon
      2. Registers tools and agents
      3. Runs agent functions with injected context
      4. Cleans up on completion or SIGINT

    Usage::

        import xkernal

        @xkernal.agent(name="my_agent", capabilities=["task"])
        async def my_agent(ctx):
            ctx.log.info("Hello from XKernal!")

        xkernal.run(my_agent)

    Returns a dict mapping agent names to their return values.
    """
    return asyncio.run(
        _run_agents(
            *agents,
            daemon_url=daemon_url,
            heartbeat_interval=heartbeat_interval,
        )
    )
