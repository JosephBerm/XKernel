"""Tests for the CLI module."""

import sys
from unittest.mock import patch

from xkernal.cli import _import_agent


def test_import_agent_bad_format():
    with patch.object(sys, "exit") as mock_exit:
        mock_exit.side_effect = SystemExit(1)
        try:
            _import_agent("no_colon_here")
        except SystemExit:
            pass
        mock_exit.assert_called_with(1)


def test_import_agent_missing_module():
    with patch.object(sys, "exit") as mock_exit:
        mock_exit.side_effect = SystemExit(1)
        try:
            _import_agent("nonexistent_module_xyz:agent")
        except SystemExit:
            pass
        mock_exit.assert_called_with(1)
