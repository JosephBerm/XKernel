"""XKernal SDK Demo — Two agents exchanging messages via IPC.

Usage:
    # Start cs-daemon first:
    cargo run -p cs-daemon

    # Then run this demo:
    xkernal run examples.sdk_demo.multi_agent:producer examples.sdk_demo.multi_agent:consumer

    # Or run directly:
    python -m examples.sdk_demo.multi_agent
"""

import asyncio
import xkernal


# ── Tools ────────────────────────────────────────────────────────────────────

@xkernal.tool(name="generate_data", effect_class="read_only")
def generate_data(topic: str, count: int = 3) -> list[str]:
    """Generate synthetic data items for a topic."""
    return [f"{topic}-item-{i}" for i in range(count)]


@xkernal.tool(name="process_item", effect_class="write_reversible")
def process_item(item: str) -> dict:
    """Process a single data item."""
    return {"processed": item.upper(), "length": len(item)}


# ── Agents ───────────────────────────────────────────────────────────────────

@xkernal.agent(
    name="producer",
    capabilities=["task", "tool", "channel"],
    tools=[generate_data],
)
async def producer(ctx: xkernal.AgentContext):
    """Produces data and sends it to the consumer."""
    ctx.log.info("Producer starting")

    # Generate data using our tool
    items = generate_data("ai-research", count=5)
    ctx.log.info(f"Generated {len(items)} items")

    # Send each item to the consumer
    for item in items:
        await ctx.send("consumer-id", {"type": "item", "data": item})
        ctx.log.info(f"Sent: {item}")

    # Signal done
    await ctx.send("consumer-id", {"type": "done"})
    ctx.log.info("Producer finished")


@xkernal.agent(
    name="consumer",
    capabilities=["task", "tool", "channel"],
    tools=[process_item],
)
async def consumer(ctx: xkernal.AgentContext):
    """Receives data from the producer and processes it."""
    ctx.log.info("Consumer starting")
    results = []

    while True:
        msg = await ctx.receive("producer-id")

        if isinstance(msg, dict) and msg.get("type") == "done":
            ctx.log.info("Received done signal")
            break

        if isinstance(msg, dict) and msg.get("type") == "item":
            result = process_item(msg["data"])
            results.append(result)
            ctx.log.info(f"Processed: {result}")

    ctx.log.info(f"Consumer finished — processed {len(results)} items")
    return results


# ── Entry point ──────────────────────────────────────────────────────────────

if __name__ == "__main__":
    xkernal.run(producer, consumer)
