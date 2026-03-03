"""Tests for @tool decorator and schema extraction."""

import json
from xkernal.tool import Tool, tool, extract_input_schema
from xkernal.types import EffectClass


def test_tool_decorator_basic():
    @tool(name="search")
    def search(query: str) -> str:
        return query

    assert isinstance(search, Tool)
    assert search.name == "search"
    assert search.config.effect_class == EffectClass.READ_ONLY


def test_tool_defaults_to_function_name():
    @tool()
    def my_tool():
        pass

    assert my_tool.name == "my_tool"


def test_tool_effect_class():
    @tool(effect_class="write_irreversible")
    def dangerous():
        pass

    assert dangerous.config.effect_class == EffectClass.WRITE_IRREVERSIBLE


def test_tool_description_from_docstring():
    @tool()
    def my_tool():
        """Search the web for information."""
        pass

    assert my_tool.config.description == "Search the web for information."


def test_schema_extraction_basic():
    def fn(query: str, limit: int = 10) -> list:
        pass

    schema_str = extract_input_schema(fn)
    schema = json.loads(schema_str)
    assert schema["type"] == "object"
    assert "query" in schema["properties"]
    assert schema["properties"]["query"]["type"] == "string"
    assert schema["properties"]["limit"]["type"] == "integer"
    assert "query" in schema["required"]
    assert "limit" not in schema["required"]


def test_schema_extraction_skips_ctx():
    def fn(ctx, query: str) -> str:
        pass

    schema_str = extract_input_schema(fn)
    schema = json.loads(schema_str)
    assert "ctx" not in schema.get("properties", {})
    assert "query" in schema["properties"]


def test_schema_extraction_empty():
    def fn():
        pass

    assert extract_input_schema(fn) == "{}"


def test_schema_extraction_complex_types():
    def fn(items: list[str], config: dict) -> dict:
        pass

    schema_str = extract_input_schema(fn)
    schema = json.loads(schema_str)
    assert schema["properties"]["items"]["type"] == "array"
    assert schema["properties"]["config"]["type"] == "object"


def test_tool_config_to_dict():
    @tool(name="search", effect_class="read_only")
    def search(query: str) -> str:
        """Search stuff."""
        return query

    d = search.config.to_dict()
    assert d["name"] == "search"
    assert d["effect_class"] == "read_only"
    assert "input_schema" in d
    assert json.loads(d["input_schema"])["properties"]["query"]["type"] == "string"
