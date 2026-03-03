"""CLI entry point — ``xkernal run module:agent``.

Provides a minimal CLI for running agents, checking daemon status,
and tailing logs.
"""

from __future__ import annotations

import argparse
import asyncio
import importlib
import sys

from ._version import __version__


def _import_agent(spec: str):
    """Import an agent from a 'module:attribute' spec.

    Example: 'myapp.agents:researcher' imports myapp.agents and
    returns the 'researcher' attribute.
    """
    if ":" not in spec:
        print(f"Error: expected 'module:agent' format, got '{spec}'", file=sys.stderr)
        sys.exit(1)

    module_path, attr_name = spec.rsplit(":", 1)
    try:
        mod = importlib.import_module(module_path)
    except ImportError as e:
        print(f"Error: cannot import module '{module_path}': {e}", file=sys.stderr)
        sys.exit(1)

    agent = getattr(mod, attr_name, None)
    if agent is None:
        print(f"Error: module '{module_path}' has no attribute '{attr_name}'", file=sys.stderr)
        sys.exit(1)

    return agent


def cmd_run(args: argparse.Namespace) -> None:
    """Handle 'xkernal run module:agent [module:agent ...]'."""
    from .agent import Agent
    from .runtime import run

    agents = []
    for spec in args.agents:
        obj = _import_agent(spec)
        if not isinstance(obj, Agent):
            print(f"Error: '{spec}' is not an @agent-decorated function", file=sys.stderr)
            sys.exit(1)
        agents.append(obj)

    if not agents:
        print("Error: no agents specified", file=sys.stderr)
        sys.exit(1)

    results = run(*agents, daemon_url=args.daemon_url)
    for name, result in results.items():
        if isinstance(result, Exception):
            print(f"  {name}: FAILED — {result}")
        else:
            print(f"  {name}: OK")


def cmd_status(args: argparse.Namespace) -> None:
    """Handle 'xkernal status'."""
    from .client import DaemonClient

    async def _status():
        async with DaemonClient(base_url=args.daemon_url) as client:
            health = await client.health()
            print(f"Daemon: {health.get('status', 'unknown')}")
            print(f"Version: {health.get('version', '?')}")
            print(f"Uptime: {health.get('uptime_seconds', 0):.1f}s")

            agents = await client.list_agents()
            print(f"Agents: {agents.get('total', 0)}")
            for a in agents.get("agents", []):
                print(f"  [{a['state']}] {a['name']} ({a['id'][:8]}...)")

    asyncio.run(_status())


def cmd_agents(args: argparse.Namespace) -> None:
    """Handle 'xkernal agents'."""
    from .client import DaemonClient

    async def _agents():
        async with DaemonClient(base_url=args.daemon_url) as client:
            resp = await client.list_agents()
            for a in resp.get("agents", []):
                pid = a.get("pid") or "-"
                print(f"  {a['id'][:8]}  {a['state']:12s}  pid={pid:>6}  {a['name']}")

    asyncio.run(_agents())


def cmd_logs(args: argparse.Namespace) -> None:
    """Handle 'xkernal logs <agent_id>'."""
    from .client import DaemonClient

    async def _logs():
        async with DaemonClient(base_url=args.daemon_url) as client:
            resp = await client.get_agent_logs(args.agent_id)
            for line in resp.get("lines", []):
                ts = line.get("timestamp", "")
                stream = line.get("stream", "")
                msg = line.get("message", "")
                print(f"[{ts}] [{stream}] {msg}")

    asyncio.run(_logs())


def main() -> None:
    """CLI entry point for ``xkernal``."""
    parser = argparse.ArgumentParser(
        prog="xkernal",
        description="XKernal Python SDK CLI",
    )
    parser.add_argument("--version", action="version", version=f"xkernal {__version__}")
    parser.add_argument(
        "--daemon-url",
        default=None,
        help="cs-daemon URL (default: $CS_DAEMON_URL or http://127.0.0.1:7600)",
    )

    sub = parser.add_subparsers(dest="command")

    # xkernal run
    run_parser = sub.add_parser("run", help="Run one or more agents")
    run_parser.add_argument("agents", nargs="+", help="module:agent specs")
    run_parser.set_defaults(func=cmd_run)

    # xkernal status
    status_parser = sub.add_parser("status", help="Show daemon status")
    status_parser.set_defaults(func=cmd_status)

    # xkernal agents
    agents_parser = sub.add_parser("agents", help="List registered agents")
    agents_parser.set_defaults(func=cmd_agents)

    # xkernal logs
    logs_parser = sub.add_parser("logs", help="Show agent logs")
    logs_parser.add_argument("agent_id", help="Agent ID")
    logs_parser.set_defaults(func=cmd_logs)

    args = parser.parse_args()
    if not hasattr(args, "func"):
        parser.print_help()
        sys.exit(1)

    try:
        args.func(args)
    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
