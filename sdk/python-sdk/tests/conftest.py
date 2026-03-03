"""Shared pytest fixtures — mock daemon via respx."""

from __future__ import annotations

import pytest
import respx
import httpx

DAEMON_URL = "http://127.0.0.1:7600"


@pytest.fixture
def mock_daemon():
    """respx mock that simulates all cs-daemon endpoints."""
    with respx.mock(base_url=DAEMON_URL, assert_all_called=False) as rsps:
        # Health
        rsps.get("/healthz").respond(json={
            "status": "ok",
            "version": "0.1.0-test",
            "uptime_seconds": 42.0,
        })
        rsps.get("/readyz").respond(json={"status": "ready"})

        # Agents
        rsps.get("/api/v1/agents").respond(json={"agents": [], "total": 0})
        rsps.post("/api/v1/agents").respond(json={
            "id": "agent-001",
            "name": "test-agent",
            "framework": "custom",
            "state": "Created",
            "pid": None,
            "created_at": "2026-01-01T00:00:00Z",
            "started_at": None,
            "uptime_seconds": None,
            "restart_count": 0,
            "capabilities": ["task"],
            "task_phase": "Pending",
            "scheduler_position": 0,
        })
        rsps.get("/api/v1/agents/agent-001").respond(json={
            "id": "agent-001",
            "name": "test-agent",
            "framework": "custom",
            "state": "Running",
            "pid": 1234,
            "created_at": "2026-01-01T00:00:00Z",
            "started_at": "2026-01-01T00:00:01Z",
            "uptime_seconds": 10.0,
            "restart_count": 0,
            "capabilities": ["task"],
            "task_phase": "Running",
            "scheduler_position": 0,
        })
        rsps.delete("/api/v1/agents/agent-001").respond(json={"status": "deleted"})
        rsps.post("/api/v1/agents/agent-001/signal").respond(json={"status": "signalled"})
        rsps.get("/api/v1/agents/agent-001/logs").respond(json={
            "agent_id": "agent-001",
            "lines": [
                {"timestamp": "2026-01-01T00:00:01Z", "stream": "stdout", "message": "hello"},
            ],
        })

        # Channels
        rsps.get("/api/v1/channels").respond(json={"channels": [], "total": 0})
        rsps.post("/api/v1/channels").respond(json={
            "id": 1,
            "sender": "agent-001",
            "receiver": "agent-002",
            "capacity": 256,
            "pending_messages": 0,
            "is_closed": False,
        })
        rsps.post("/api/v1/channels/1/send").respond(json={
            "sender": "agent-001",
            "receiver": "agent-002",
            "payload": "test",
            "sequence": 1,
            "timestamp": 1704067200,
        })
        rsps.post("/api/v1/channels/1/receive").respond(json={
            "sender": "agent-001",
            "receiver": "agent-002",
            "payload": '{"msg": "hello"}',
            "sequence": 1,
            "timestamp": 1704067200,
        })

        # Memory
        rsps.get("/api/v1/memory").respond(json={
            "total_pages": 1024,
            "allocated_pages": 0,
            "free_pages": 1024,
            "active_allocations": 0,
            "page_size_bytes": 4096,
        })
        rsps.post("/api/v1/memory/allocate").respond(json={
            "allocation_id": 1,
            "pages": 10,
            "size_bytes": 40960,
            "owner_ct_id": 1,
        })
        rsps.post("/api/v1/memory/free").respond(json={"status": "freed"})

        # Tools
        rsps.get("/api/v1/tools").respond(json={"tools": [], "total": 0})
        rsps.post("/api/v1/tools").respond(json={
            "binding_id": "tool-001",
            "name": "test-tool",
            "effect_class": "read_only",
            "agent_id": "agent-001",
        })
        rsps.delete("/api/v1/tools/tool-001").respond(json={"status": "deleted"})

        # System
        rsps.get("/api/v1/metrics").respond(json={
            "uptime_seconds": 42.0,
            "agents": {"total_created": 0, "active": 0, "running": 0, "stopped": 0, "failed": 0},
            "scheduler": {"queue_depth": 0, "total_scheduled": 0, "total_completed": 0},
            "channels": {"active_channels": 0, "total_messages_sent": 0, "total_messages_received": 0},
            "memory": {"total_allocations": 0, "active_allocations": 0, "total_bytes_allocated": 0},
            "tools": {"registered_tools": 0, "total_invocations": 0},
            "telemetry": {"total_events": 0, "events_per_type": {}},
        })
        rsps.get("/api/v1/events").respond(json={"events": [], "total": 0})

        yield rsps
