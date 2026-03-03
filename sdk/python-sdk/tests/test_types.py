"""Tests for xkernal.types."""

from xkernal.types import (
    AgentConfig,
    AgentState,
    AllocateMemoryConfig,
    Capability,
    ChannelConfig,
    EffectClass,
    FrameworkType,
    RestartPolicy,
    ToolConfig,
)


def test_restart_policy_values():
    assert RestartPolicy.NEVER.value == "never"
    assert RestartPolicy.ON_FAILURE.value == "on_failure"
    assert RestartPolicy.ALWAYS.value == "always"


def test_effect_class_values():
    assert EffectClass.READ_ONLY.value == "read_only"
    assert EffectClass.WRITE_REVERSIBLE.value == "write_reversible"
    assert EffectClass.WRITE_IRREVERSIBLE.value == "write_irreversible"


def test_framework_type_values():
    assert FrameworkType.LANGCHAIN.value == "langchain"
    assert FrameworkType.CREWAI.value == "crewai"
    assert FrameworkType.CUSTOM.value == "custom"


def test_agent_state_values():
    assert AgentState.CREATED.value == "Created"
    assert AgentState.RUNNING.value == "Running"
    assert AgentState.FAILED.value == "Failed"


def test_capability_values():
    assert Capability.TASK.value == "task"
    assert Capability.MEMORY.value == "memory"
    assert Capability.CHANNEL.value == "channel"


def test_agent_config_defaults():
    cfg = AgentConfig(name="test")
    assert cfg.name == "test"
    assert cfg.framework == FrameworkType.CUSTOM
    assert cfg.priority == 128
    assert cfg.restart_policy == RestartPolicy.NEVER
    assert cfg.max_retries == 3
    assert cfg.capabilities == []
    assert cfg.env == {}


def test_agent_config_to_dict():
    cfg = AgentConfig(
        name="researcher",
        framework=FrameworkType.LANGCHAIN,
        capabilities=[Capability.TASK, Capability.TOOL],
        priority=200,
    )
    d = cfg.to_dict()
    assert d["name"] == "researcher"
    assert d["framework"] == "langchain"
    assert d["capabilities"] == ["task", "tool"]
    assert d["priority"] == 200
    assert "entrypoint" not in d
    assert "working_dir" not in d


def test_agent_config_to_dict_with_entrypoint():
    cfg = AgentConfig(name="worker", entrypoint="python agent.py", working_dir="/app")
    d = cfg.to_dict()
    assert d["entrypoint"] == "python agent.py"
    assert d["working_dir"] == "/app"


def test_tool_config_to_dict():
    cfg = ToolConfig(name="search", effect_class=EffectClass.READ_ONLY)
    d = cfg.to_dict()
    assert d["name"] == "search"
    assert d["effect_class"] == "read_only"
    assert d["input_schema"] == "{}"
    assert "agent_id" not in d


def test_channel_config_to_dict():
    cfg = ChannelConfig(sender="a", receiver="b", capacity=100)
    d = cfg.to_dict()
    assert d["sender"] == "a"
    assert d["receiver"] == "b"
    assert d["capacity"] == 100


def test_allocate_memory_config():
    cfg = AllocateMemoryConfig(pages=10, owner_ct_id=1)
    d = cfg.to_dict()
    assert d["pages"] == 10
    assert d["owner_ct_id"] == 1
