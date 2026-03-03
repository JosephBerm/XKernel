"""Tests for xkernal.client — DaemonClient with mocked HTTP."""

import pytest
from xkernal.client import DaemonClient
from xkernal.types import AgentConfig, ChannelConfig, ToolConfig, AllocateMemoryConfig
from xkernal.errors import CsciError


@pytest.mark.asyncio
async def test_health(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.health()
        assert resp["status"] == "ok"


@pytest.mark.asyncio
async def test_readiness(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.readiness()
        assert resp["status"] == "ready"


@pytest.mark.asyncio
async def test_create_agent(mock_daemon):
    async with DaemonClient() as client:
        cfg = AgentConfig(name="test-agent")
        resp = await client.create_agent(cfg)
        assert resp["id"] == "agent-001"
        assert resp["name"] == "test-agent"


@pytest.mark.asyncio
async def test_list_agents(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.list_agents()
        assert resp["total"] == 0
        assert resp["agents"] == []


@pytest.mark.asyncio
async def test_get_agent(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.get_agent("agent-001")
        assert resp["state"] == "Running"
        assert resp["pid"] == 1234


@pytest.mark.asyncio
async def test_delete_agent(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.delete_agent("agent-001")
        assert resp["status"] == "deleted"


@pytest.mark.asyncio
async def test_signal_agent(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.signal_agent("agent-001", "stop", "test")
        assert resp["status"] == "signalled"


@pytest.mark.asyncio
async def test_agent_logs(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.get_agent_logs("agent-001")
        assert len(resp["lines"]) == 1
        assert resp["lines"][0]["stream"] == "stdout"


@pytest.mark.asyncio
async def test_create_channel(mock_daemon):
    async with DaemonClient() as client:
        cfg = ChannelConfig(sender="agent-001", receiver="agent-002")
        resp = await client.create_channel(cfg)
        assert resp["id"] == 1


@pytest.mark.asyncio
async def test_send_receive_message(mock_daemon):
    async with DaemonClient() as client:
        send_resp = await client.send_message(1, "test payload")
        assert send_resp["sequence"] == 1

        recv_resp = await client.receive_message(1)
        assert "payload" in recv_resp


@pytest.mark.asyncio
async def test_memory_stats(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.memory_stats()
        assert resp["total_pages"] == 1024


@pytest.mark.asyncio
async def test_allocate_free_memory(mock_daemon):
    async with DaemonClient() as client:
        cfg = AllocateMemoryConfig(pages=10, owner_ct_id=1)
        alloc = await client.allocate_memory(cfg)
        assert alloc["allocation_id"] == 1
        assert alloc["size_bytes"] == 40960

        free_resp = await client.free_memory(1)
        assert free_resp["status"] == "freed"


@pytest.mark.asyncio
async def test_register_tool(mock_daemon):
    async with DaemonClient() as client:
        cfg = ToolConfig(name="test-tool")
        resp = await client.register_tool(cfg)
        assert resp["binding_id"] == "tool-001"


@pytest.mark.asyncio
async def test_unregister_tool(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.unregister_tool("tool-001")
        assert resp["status"] == "deleted"


@pytest.mark.asyncio
async def test_metrics(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.metrics()
        assert resp["uptime_seconds"] == 42.0


@pytest.mark.asyncio
async def test_events(mock_daemon):
    async with DaemonClient() as client:
        resp = await client.events()
        assert resp["total"] == 0
