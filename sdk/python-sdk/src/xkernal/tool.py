"""@tool decorator — declares a tool binding for XKernal.

Automatically extracts a JSON schema from the function's type hints
to register with the daemon's tool registry.
"""

from __future__ import annotations

import inspect
import json
from dataclasses import dataclass
from typing import Any, Callable, get_type_hints

from .types import EffectClass, ToolConfig


# ── JSON schema extraction from type hints ───────────────────────────────────

_PY_TYPE_TO_JSON: dict[type, str] = {
    str: "string",
    int: "integer",
    float: "number",
    bool: "boolean",
    list: "array",
    dict: "object",
}


def _type_to_json_schema(t: Any) -> dict:
    """Convert a Python type annotation to a basic JSON schema."""
    if t is inspect.Parameter.empty or t is Any:
        return {}

    origin = getattr(t, "__origin__", None)
    if origin is list:
        args = getattr(t, "__args__", ())
        items = _type_to_json_schema(args[0]) if args else {}
        return {"type": "array", "items": items}
    if origin is dict:
        return {"type": "object"}

    json_type = _PY_TYPE_TO_JSON.get(t)
    if json_type:
        return {"type": json_type}

    return {}


def extract_input_schema(fn: Callable) -> str:
    """Extract a JSON schema string from a function's parameters.

    Skips 'self' and 'ctx' parameters (convention for agent context).
    """
    try:
        hints = get_type_hints(fn)
    except Exception:
        hints = {}

    sig = inspect.signature(fn)
    properties: dict[str, Any] = {}
    required: list[str] = []

    for param_name, param in sig.parameters.items():
        if param_name in ("self", "ctx", "context"):
            continue
        schema = _type_to_json_schema(hints.get(param_name, inspect.Parameter.empty))
        if schema:
            properties[param_name] = schema
        else:
            properties[param_name] = {}
        if param.default is inspect.Parameter.empty:
            required.append(param_name)

    if not properties:
        return "{}"

    schema_obj: dict[str, Any] = {
        "type": "object",
        "properties": properties,
    }
    if required:
        schema_obj["required"] = required
    return json.dumps(schema_obj)


# ── Tool dataclass and decorator ─────────────────────────────────────────────


@dataclass
class Tool:
    """A tool definition created by the @tool decorator."""
    fn: Callable[..., Any]
    config: ToolConfig

    @property
    def name(self) -> str:
        return self.config.name

    async def __call__(self, *args: Any, **kwargs: Any) -> Any:
        from ._sync import ensure_async
        fn = ensure_async(self.fn)
        return await fn(*args, **kwargs)


def tool(
    name: str | None = None,
    *,
    description: str = "",
    effect_class: EffectClass | str = EffectClass.READ_ONLY,
) -> Callable[[Callable], Tool]:
    """Decorator that declares a tool binding.

    Usage::

        @tool(name="web_search", effect_class="read_only")
        def web_search(query: str, max_results: int = 10) -> list[dict]:
            ...

    The JSON input schema is automatically extracted from type hints.
    """

    def decorator(fn: Callable) -> Tool:
        tool_name = name or fn.__name__
        ec = EffectClass(effect_class) if isinstance(effect_class, str) else effect_class
        desc = description or (fn.__doc__ or "").strip().split("\n")[0]

        input_schema = extract_input_schema(fn)

        config = ToolConfig(
            name=tool_name,
            description=desc,
            input_schema=input_schema,
            effect_class=ec,
        )
        return Tool(fn=fn, config=config)

    return decorator
