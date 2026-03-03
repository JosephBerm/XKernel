"""Sync/async bridge utilities.

Allows @agent-decorated functions to be either sync or async.
Sync functions are automatically wrapped with asyncio.to_thread
so they don't block the event loop.
"""

from __future__ import annotations

import asyncio
import functools
import inspect
from typing import Any, Callable, Coroutine


def is_async(fn: Callable) -> bool:
    """Check if a function is async (coroutine function)."""
    return inspect.iscoroutinefunction(fn)


def ensure_async(fn: Callable[..., Any]) -> Callable[..., Coroutine[Any, Any, Any]]:
    """Wrap a sync function to run in a thread; pass through async functions."""
    if is_async(fn):
        return fn

    @functools.wraps(fn)
    async def wrapper(*args: Any, **kwargs: Any) -> Any:
        return await asyncio.to_thread(fn, *args, **kwargs)

    return wrapper


def run_sync(coro: Coroutine[Any, Any, Any]) -> Any:
    """Run an async coroutine synchronously.

    Uses the running loop if available, otherwise creates one.
    """
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = None

    if loop and loop.is_running():
        import concurrent.futures
        with concurrent.futures.ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(asyncio.run, coro)
            return future.result()
    else:
        return asyncio.run(coro)
