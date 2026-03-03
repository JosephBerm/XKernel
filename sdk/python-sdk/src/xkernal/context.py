"""AgentContext — injected into agent functions at runtime.

Provides a clean interface for agents to interact with the kernel:
send/receive IPC messages, invoke tools, emit telemetry, and log.
"""

from __future__ import annotations

import json
from contextvars import ContextVar
from typing import Any

from .client import DaemonClient
from .logging import get_logger

# ContextVar so each agent task gets its own context
_current_context: ContextVar[AgentContext | None] = ContextVar("xkernal_ctx", default=None)


def current_context() -> AgentContext:
    """Get the current agent's context. Raises if not inside xkernal.run()."""
    ctx = _current_context.get()
    if ctx is None:
        raise RuntimeError(
            "No active AgentContext. Are you inside an agent function "
            "invoked by xkernal.run()?"
        )
    return ctx


class AgentContext:
    """Runtime context injected into agent functions.

    Provides methods for IPC, tool invocation, and telemetry —
    all routed through the daemon client.
    """

    def __init__(
        self,
        agent_id: str,
        agent_name: str,
        client: DaemonClient,
    ) -> None:
        self.agent_id = agent_id
        self.agent_name = agent_name
        self._client = client
        self._channel_cache: dict[tuple[str, str], int] = {}
        self.log = get_logger()

    async def send(self, target_agent_id: str, payload: Any) -> dict:
        """Send a message to another agent via IPC.

        Creates a channel lazily on first send to a given target.
        """
        channel_id = await self._get_or_create_channel(self.agent_id, target_agent_id)
        payload_str = json.dumps(payload) if not isinstance(payload, str) else payload
        return await self._client.send_message(channel_id, payload_str)

    async def receive(self, sender_agent_id: str) -> Any:
        """Receive a message from another agent.

        Returns the parsed payload (JSON-decoded if possible).
        """
        channel_id = await self._get_or_create_channel(sender_agent_id, self.agent_id)
        msg = await self._client.receive_message(channel_id)
        payload = msg.get("payload", "")
        try:
            return json.loads(payload)
        except (json.JSONDecodeError, TypeError):
            return payload

    async def invoke_tool(self, tool_name: str, inputs: dict | None = None) -> Any:
        """Invoke a registered tool by name.

        Note: In Phase 0, tools are local function calls registered
        with the daemon for tracking. Full remote invocation comes later.
        """
        self.log.info(f"Invoking tool: {tool_name}")
        # Tools are currently local — the runtime resolves them
        raise NotImplementedError(
            f"Remote tool invocation not yet available. "
            f"Tool '{tool_name}' should be called directly."
        )

    async def emit_trace(self, event_type: str, details: str = "") -> None:
        """Emit a telemetry trace event through structured logging."""
        self.log.info(
            f"[trace] {event_type}: {details}",
            extra={"event_type": event_type},
        )

    async def get_status(self) -> dict:
        """Get this agent's current status from the daemon."""
        return await self._client.get_agent(self.agent_id)

    async def _get_or_create_channel(self, sender: str, receiver: str) -> int:
        """Get or lazily create an IPC channel between two agents."""
        key = (sender, receiver)
        if key in self._channel_cache:
            return self._channel_cache[key]

        # Check existing channels
        channels_resp = await self._client.list_channels()
        for ch in channels_resp.get("channels", []):
            if ch["sender"] == sender and ch["receiver"] == receiver:
                self._channel_cache[key] = ch["id"]
                return ch["id"]

        # Create new channel
        from .types import ChannelConfig
        config = ChannelConfig(sender=sender, receiver=receiver)
        result = await self._client.create_channel(config)
        channel_id = result["id"]
        self._channel_cache[key] = channel_id
        return channel_id
