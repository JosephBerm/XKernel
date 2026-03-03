"""Async HTTP client for the cs-daemon REST API.

One method per daemon endpoint (20 total), matching routes.rs exactly.
Configuration via CS_DAEMON_URL environment variable.
"""

from __future__ import annotations

import os
from typing import Any

import httpx

from .errors import CsciError, CsciErrorCode, DaemonConnectionError
from .types import AgentConfig, AllocateMemoryConfig, ChannelConfig, ToolConfig

DEFAULT_DAEMON_URL = "http://127.0.0.1:7600"


def _daemon_url() -> str:
    return os.environ.get("CS_DAEMON_URL", DEFAULT_DAEMON_URL)


def _raise_for_error(resp: httpx.Response) -> None:
    """Convert daemon HTTP error responses into CsciError exceptions."""
    if resp.is_success:
        return
    try:
        body = resp.json()
        detail = body.get("error", resp.text)
    except Exception:
        detail = resp.text

    code_map = {
        400: CsciErrorCode.CS_EINVAL,
        403: CsciErrorCode.CS_EPERM,
        404: CsciErrorCode.CS_ENOENT,
        408: CsciErrorCode.CS_ETIMEOUT,
        409: CsciErrorCode.CS_EEXIST,
        410: CsciErrorCode.CS_ECLOSED,
        500: CsciErrorCode.CS_EUNIMPL,
    }
    code = code_map.get(resp.status_code, CsciErrorCode.CS_EINVAL)
    raise CsciError(code, detail, resp.status_code)


class DaemonClient:
    """Async client for every cs-daemon endpoint.

    Usage::

        async with DaemonClient() as client:
            health = await client.health()
            agents = await client.list_agents()
    """

    def __init__(self, base_url: str | None = None, timeout: float = 30.0) -> None:
        self._base_url = (base_url or _daemon_url()).rstrip("/")
        self._timeout = timeout
        self._http: httpx.AsyncClient | None = None

    async def _client(self) -> httpx.AsyncClient:
        if self._http is None or self._http.is_closed:
            self._http = httpx.AsyncClient(
                base_url=self._base_url,
                timeout=self._timeout,
            )
        return self._http

    async def close(self) -> None:
        if self._http and not self._http.is_closed:
            await self._http.aclose()

    async def __aenter__(self) -> DaemonClient:
        return self

    async def __aexit__(self, *exc: Any) -> None:
        await self.close()

    # ── Helpers ──────────────────────────────────────────────────────────────

    async def _get(self, path: str) -> Any:
        try:
            c = await self._client()
            resp = await c.get(path)
        except httpx.ConnectError as e:
            raise DaemonConnectionError(str(e)) from e
        _raise_for_error(resp)
        return resp.json()

    async def _post(self, path: str, json: Any = None) -> Any:
        try:
            c = await self._client()
            resp = await c.post(path, json=json)
        except httpx.ConnectError as e:
            raise DaemonConnectionError(str(e)) from e
        _raise_for_error(resp)
        return resp.json()

    async def _delete(self, path: str) -> Any:
        try:
            c = await self._client()
            resp = await c.delete(path)
        except httpx.ConnectError as e:
            raise DaemonConnectionError(str(e)) from e
        _raise_for_error(resp)
        return resp.json()

    # ── Health probes (2) ────────────────────────────────────────────────────

    async def health(self) -> dict:
        """GET /healthz"""
        return await self._get("/healthz")

    async def readiness(self) -> dict:
        """GET /readyz"""
        return await self._get("/readyz")

    # ── Agent management (5) ─────────────────────────────────────────────────

    async def list_agents(self) -> dict:
        """GET /api/v1/agents → {agents: [...], total: N}"""
        return await self._get("/api/v1/agents")

    async def create_agent(self, config: AgentConfig) -> dict:
        """POST /api/v1/agents → AgentResponse"""
        return await self._post("/api/v1/agents", json=config.to_dict())

    async def get_agent(self, agent_id: str) -> dict:
        """GET /api/v1/agents/:id → AgentResponse"""
        return await self._get(f"/api/v1/agents/{agent_id}")

    async def delete_agent(self, agent_id: str) -> dict:
        """DELETE /api/v1/agents/:id"""
        return await self._delete(f"/api/v1/agents/{agent_id}")

    async def signal_agent(self, agent_id: str, signal: str, reason: str | None = None) -> dict:
        """POST /api/v1/agents/:id/signal"""
        body: dict[str, Any] = {"signal": signal}
        if reason:
            body["reason"] = reason
        return await self._post(f"/api/v1/agents/{agent_id}/signal", json=body)

    async def get_agent_logs(self, agent_id: str) -> dict:
        """GET /api/v1/agents/:id/logs → {agent_id, lines: [...]}"""
        return await self._get(f"/api/v1/agents/{agent_id}/logs")

    # ── IPC channels (4) ─────────────────────────────────────────────────────

    async def list_channels(self) -> dict:
        """GET /api/v1/channels → {channels: [...], total: N}"""
        return await self._get("/api/v1/channels")

    async def create_channel(self, config: ChannelConfig) -> dict:
        """POST /api/v1/channels → ChannelResponse"""
        return await self._post("/api/v1/channels", json=config.to_dict())

    async def send_message(self, channel_id: int, payload: str) -> dict:
        """POST /api/v1/channels/:id/send"""
        return await self._post(
            f"/api/v1/channels/{channel_id}/send",
            json={"payload": payload},
        )

    async def receive_message(self, channel_id: int) -> dict:
        """POST /api/v1/channels/:id/receive → MessageResponse"""
        return await self._post(f"/api/v1/channels/{channel_id}/receive")

    # ── Memory management (3) ────────────────────────────────────────────────

    async def memory_stats(self) -> dict:
        """GET /api/v1/memory → MemoryStatsResponse"""
        return await self._get("/api/v1/memory")

    async def allocate_memory(self, config: AllocateMemoryConfig) -> dict:
        """POST /api/v1/memory/allocate → AllocationResponse"""
        return await self._post("/api/v1/memory/allocate", json=config.to_dict())

    async def free_memory(self, allocation_id: int) -> dict:
        """POST /api/v1/memory/free"""
        return await self._post("/api/v1/memory/free", json={"allocation_id": allocation_id})

    # ── Tool registry (3) ────────────────────────────────────────────────────

    async def list_tools(self) -> dict:
        """GET /api/v1/tools → {tools: [...], total: N}"""
        return await self._get("/api/v1/tools")

    async def register_tool(self, config: ToolConfig) -> dict:
        """POST /api/v1/tools → ToolResponse"""
        return await self._post("/api/v1/tools", json=config.to_dict())

    async def unregister_tool(self, tool_id: str) -> dict:
        """DELETE /api/v1/tools/:id"""
        return await self._delete(f"/api/v1/tools/{tool_id}")

    # ── System telemetry (2) ─────────────────────────────────────────────────

    async def metrics(self) -> dict:
        """GET /api/v1/metrics → MetricsResponse"""
        return await self._get("/api/v1/metrics")

    async def events(self) -> dict:
        """GET /api/v1/events → {events: [...], total: N}"""
        return await self._get("/api/v1/events")
