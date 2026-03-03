#!/usr/bin/env python3
"""
XKernal Long-Running Agent — demonstrates lifecycle management, restart policies,
and inter-agent communication patterns.

This agent runs continuously, simulating an AI agent that:
- Processes incoming tasks
- Reports health status
- Handles signals gracefully
- Can be checkpointed and resumed

Usage:
    cs-ctl agent create worker --command "python examples/hello-agent/long_running_agent.py" \
        --restart on_failure
"""

import sys
import time
import json
import os
import signal
from datetime import datetime

running = True

def handle_signal(signum, frame):
    global running
    log(f"Received signal {signum}, initiating graceful shutdown...")
    running = False

def log(message: str):
    entry = {
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "agent": "long-running-agent",
        "pid": os.getpid(),
        "message": message
    }
    print(json.dumps(entry), flush=True)

def main():
    global running

    # Register signal handlers for graceful shutdown
    signal.signal(signal.SIGTERM, handle_signal)
    signal.signal(signal.SIGINT, handle_signal)

    log("Long-running agent starting...")
    log(f"PID: {os.getpid()}, Python: {sys.version}")

    cycle = 0
    while running:
        cycle += 1
        log(f"Work cycle {cycle}: processing...")

        # Simulate AI inference work
        time.sleep(2)

        # Report status every cycle
        status = {
            "cycle": cycle,
            "memory_mb": 128 + (cycle * 2),
            "tasks_completed": cycle * 3,
            "health": "ok"
        }
        log(f"Status: {json.dumps(status)}")

        # Simulate occasional memory pressure (for testing resource monitoring)
        if cycle % 10 == 0:
            log(f"Memory pressure detected at cycle {cycle}, triggering GC...")
            time.sleep(0.5)

    log(f"Agent shut down gracefully after {cycle} cycles")

if __name__ == "__main__":
    main()
