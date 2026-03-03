"""Tests for @agent decorator."""

from xkernal.agent import Agent, agent
from xkernal.types import Capability, FrameworkType, RestartPolicy


def test_agent_decorator_basic():
    @agent(name="test")
    async def my_agent(ctx):
        pass

    assert isinstance(my_agent, Agent)
    assert my_agent.name == "test"
    assert my_agent.config.framework == FrameworkType.CUSTOM


def test_agent_decorator_defaults_to_function_name():
    @agent()
    async def researcher(ctx):
        pass

    assert researcher.name == "researcher"


def test_agent_decorator_full_config():
    @agent(
        name="worker",
        framework="langchain",
        capabilities=["task", "tool", "channel"],
        priority=200,
        restart_policy="on_failure",
        max_retries=5,
    )
    async def worker(ctx):
        pass

    assert worker.config.framework == FrameworkType.LANGCHAIN
    assert worker.config.priority == 200
    assert worker.config.restart_policy == RestartPolicy.ON_FAILURE
    assert worker.config.max_retries == 5
    assert Capability.TASK in worker.config.capabilities
    assert Capability.TOOL in worker.config.capabilities


def test_agent_attach_tool():
    from xkernal.tool import tool

    @tool(name="search")
    def search(query: str) -> str:
        return query

    @agent(name="test")
    async def my_agent(ctx):
        pass

    my_agent.attach_tool(search)
    assert len(my_agent.tools) == 1
    assert my_agent.tools[0].name == "search"


def test_agent_with_tools_param():
    from xkernal.tool import tool

    @tool(name="t1")
    def t1():
        pass

    @tool(name="t2")
    def t2():
        pass

    @agent(name="test", tools=[t1, t2])
    async def my_agent(ctx):
        pass

    assert len(my_agent.tools) == 2


def test_agent_config_to_dict():
    @agent(name="test", capabilities=["task"], priority=50)
    async def my_agent(ctx):
        pass

    d = my_agent.config.to_dict()
    assert d["name"] == "test"
    assert d["priority"] == 50
    assert d["capabilities"] == ["task"]
