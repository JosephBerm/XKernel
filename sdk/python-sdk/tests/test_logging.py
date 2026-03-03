"""Tests for structured JSON logging."""

import io
import json
import logging

from xkernal.logging import JsonLineHandler, setup_logging, get_logger


def test_json_line_handler():
    stream = io.StringIO()
    handler = JsonLineHandler(agent_id="test-001", stream=stream)
    handler.setLevel(logging.DEBUG)

    logger = logging.getLogger("test_json_handler")
    logger.addHandler(handler)
    logger.setLevel(logging.DEBUG)

    logger.info("hello world")

    output = stream.getvalue()
    assert output.endswith("\n")

    entry = json.loads(output.strip())
    assert entry["level"] == "INFO"
    assert entry["agent_id"] == "test-001"
    assert entry["msg"] == "hello world"
    assert "ts" in entry

    logger.removeHandler(handler)


def test_setup_logging():
    logger = setup_logging(agent_id="agent-x", level=logging.DEBUG)
    assert logger.name == "xkernal"
    assert any(isinstance(h, JsonLineHandler) for h in logger.handlers)


def test_get_logger():
    logger = get_logger()
    assert logger.name == "xkernal"


def test_setup_logging_replaces_handler():
    setup_logging(agent_id="first")
    setup_logging(agent_id="second")

    logger = logging.getLogger("xkernal")
    json_handlers = [h for h in logger.handlers if isinstance(h, JsonLineHandler)]
    assert len(json_handlers) == 1
    assert json_handlers[0].agent_id == "second"
