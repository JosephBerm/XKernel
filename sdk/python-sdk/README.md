# XKernal Python SDK

Python SDK for XKernal — the AI-native cognitive substrate OS.

## Install

```bash
pip install xkernal
```

## Quick Start

```python
import xkernal

@xkernal.agent(name="my_agent", capabilities=["task", "tool"])
async def my_agent(ctx):
    ctx.log.info("Hello from XKernal!")

xkernal.run(my_agent)
```

## Requirements

- Python >= 3.10
- Running cs-daemon (`cargo run -p cs-daemon`)
