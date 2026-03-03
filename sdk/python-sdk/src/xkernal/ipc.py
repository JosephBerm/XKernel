"""IPC helpers — Channel class for inter-agent communication.

Wraps the daemon's channel endpoints into a clean Channel abstraction.
Most agents will use ctx.send()/ctx.receive() instead of this directly.
"""

from __future__ import annotations

import json
from typing import Any

from .client import DaemonClient
from .types import ChannelConfig


class Channel:
    """An IPC channel between two agents.

    Usage::

        channel = await Channel.create(client, sender_id, receiver_id)
        await channel.send({"key": "value"})
        msg = await channel.receive()
    """

    def __init__(self, channel_id: int, sender: str, receiver: str, client: DaemonClient) -> None:
        self.id = channel_id
        self.sender = sender
        self.receiver = receiver
        self._client = client

    @classmethod
    async def create(
        cls,
        client: DaemonClient,
        sender: str,
        receiver: str,
        capacity: int = 256,
    ) -> Channel:
        """Create a new IPC channel via the daemon."""
        config = ChannelConfig(sender=sender, receiver=receiver, capacity=capacity)
        result = await client.create_channel(config)
        return cls(
            channel_id=result["id"],
            sender=sender,
            receiver=receiver,
            client=client,
        )

    async def send(self, payload: Any) -> dict:
        """Send a message on this channel.

        Payload is JSON-serialized if not already a string.
        """
        payload_str = json.dumps(payload) if not isinstance(payload, str) else payload
        return await self._client.send_message(self.id, payload_str)

    async def receive(self) -> Any:
        """Receive the next message from this channel.

        Returns the parsed payload (JSON-decoded if possible).
        """
        msg = await self._client.receive_message(self.id)
        payload = msg.get("payload", "")
        try:
            return json.loads(payload)
        except (json.JSONDecodeError, TypeError):
            return payload

    async def info(self) -> dict:
        """Get channel status from the daemon."""
        channels = await self._client.list_channels()
        for ch in channels.get("channels", []):
            if ch["id"] == self.id:
                return ch
        return {"id": self.id, "sender": self.sender, "receiver": self.receiver}
