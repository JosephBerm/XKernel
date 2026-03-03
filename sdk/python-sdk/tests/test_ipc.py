"""Tests for IPC Channel class."""

import pytest
from xkernal.client import DaemonClient
from xkernal.ipc import Channel


@pytest.mark.asyncio
async def test_channel_create(mock_daemon):
    async with DaemonClient() as client:
        ch = await Channel.create(client, "agent-001", "agent-002")
        assert ch.id == 1
        assert ch.sender == "agent-001"
        assert ch.receiver == "agent-002"


@pytest.mark.asyncio
async def test_channel_send(mock_daemon):
    async with DaemonClient() as client:
        ch = await Channel.create(client, "agent-001", "agent-002")
        resp = await ch.send({"msg": "hello"})
        assert resp["sequence"] == 1


@pytest.mark.asyncio
async def test_channel_receive(mock_daemon):
    async with DaemonClient() as client:
        ch = await Channel.create(client, "agent-001", "agent-002")
        msg = await ch.receive()
        assert msg == {"msg": "hello"}
