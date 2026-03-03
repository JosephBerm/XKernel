"""Structured JSON logging to stdout.

The cs-daemon supervisor captures agent stdout line-by-line
(see daemon/cs-daemon/src/supervisor.rs). This module emits
one JSON object per line so the daemon can parse and index logs.
"""

from __future__ import annotations

import json
import logging
import sys
from datetime import datetime, timezone


class JsonLineHandler(logging.Handler):
    """Logging handler that emits JSON lines to stdout.

    Each line is a JSON object with fields:
      {"ts": "...", "level": "INFO", "agent_id": "...", "msg": "..."}

    The daemon's stdout reader captures these lines and stores them
    as agent log entries.
    """

    def __init__(self, agent_id: str = "", stream: Any = None) -> None:
        super().__init__()
        self.agent_id = agent_id
        self.stream = stream or sys.stdout

    def emit(self, record: logging.LogRecord) -> None:
        try:
            entry = {
                "ts": datetime.now(timezone.utc).isoformat(),
                "level": record.levelname,
                "agent_id": self.agent_id,
                "msg": self.format(record),
            }
            if record.exc_info and record.exc_info[1]:
                entry["exception"] = str(record.exc_info[1])
            line = json.dumps(entry, default=str)
            self.stream.write(line + "\n")
            self.stream.flush()
        except Exception:
            self.handleError(record)


# Needed for type hint above — import at module level causes circular
from typing import Any  # noqa: E402


def setup_logging(agent_id: str = "", level: int = logging.INFO) -> logging.Logger:
    """Configure the xkernal logger with JSON stdout output.

    Returns the configured logger. Calling this multiple times
    with different agent_ids replaces the handler.
    """
    logger = logging.getLogger("xkernal")
    logger.setLevel(level)

    # Remove existing JsonLineHandlers
    for h in logger.handlers[:]:
        if isinstance(h, JsonLineHandler):
            logger.removeHandler(h)

    handler = JsonLineHandler(agent_id=agent_id)
    handler.setLevel(level)
    logger.addHandler(handler)
    return logger


def get_logger() -> logging.Logger:
    """Get the xkernal logger (creates if needed)."""
    logger = logging.getLogger("xkernal")
    if not logger.handlers:
        setup_logging()
    return logger
