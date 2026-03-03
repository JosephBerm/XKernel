#!/usr/bin/env python3
"""
XKernal Hello Agent — a simple agent managed by cs-daemon.

This demonstrates the basic lifecycle of a managed AI agent:
1. Startup phase: Agent initializes and reports ready
2. Working phase: Agent performs tasks in a loop
3. Shutdown: Agent exits cleanly when done

Usage:
    cs-ctl agent create hello-bot --command "python examples/hello-agent/agent.py"
"""

import sys
import time
import json
import os
from datetime import datetime

def log(message: str):
    """Structured log output that cs-daemon captures."""
    entry = {
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "agent": "hello-agent",
        "message": message
    }
    print(json.dumps(entry), flush=True)

def main():
    log("Agent starting up...")
    log(f"PID: {os.getpid()}")
    log(f"Python: {sys.version}")

    # Simulate initialization (loading model, connecting to services, etc.)
    log("Phase 1: Initializing cognitive task...")
    time.sleep(1)
    log("Phase 1: Complete — ready to process")

    # Working loop — simulate AI agent doing work
    iterations = int(os.environ.get("AGENT_ITERATIONS", "5"))
    for i in range(1, iterations + 1):
        log(f"Phase 2: Processing task {i}/{iterations}")

        # Simulate thinking/inference
        time.sleep(0.5)
        result = f"Result from iteration {i}: computed value = {i * 42}"
        log(f"Phase 2: Task {i} complete — {result}")

    # Clean shutdown
    log("Phase 3: All tasks complete, shutting down gracefully")
    log(f"Total iterations: {iterations}")
    log("Agent exited successfully")

if __name__ == "__main__":
    main()
