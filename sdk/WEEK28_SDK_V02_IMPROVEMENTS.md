# XKernal SDK v0.2 Improvements — Technical Implementation Guide

**Engineer 9 | Week 28 | CSCI & SDK Development**

---

## Executive Summary

SDK v0.2 addresses Week 27 usability feedback through API clarity, enhanced error messaging, comprehensive examples, and missing utility functions. This release targets a 50% reduction in time-to-first-hello-world while maintaining API stability and MAANG-level code quality.

**Target Metrics:**
- Hello World setup time: <5 minutes (from ~10 minutes)
- TypeScript PASS rate: 88% → 95%+
- C# MARGINAL rate: 81% → 90%+
- Example coverage: 100% of SDK patterns

---

## 1. API Clarity Improvements

### 1.1 Renaming Ambiguous Functions

**Before (Week 27):**
```typescript
// Confusing: unclear if this creates or registers
async function register(config: CognitiveConfig): Promise<Substrate> {
  return this.lifecycle.initialize(config);
}

// Ambiguous: "handle" could mean process, store, or execute
async function handle(payload: Payload): Promise<Result> {
  return this.executor.process(payload);
}

// Vague: "setup" scope unclear
function setup(tools: Tool[]): void {
  this.registry.mount(tools);
}
```

**After (v0.2):**
```typescript
// Clear intent: creates and returns Substrate instance
async function createSubstrate(config: CognitiveConfig): Promise<Substrate> {
  return this.lifecycle.initialize(config);
}

// Explicit action: executes payload through pipeline
async function executePayload(payload: Payload): Promise<Result> {
  return this.executor.process(payload);
}

// Precise scope: registers tools for current session
function registerTools(tools: Tool[]): void {
  this.registry.mount(tools);
}
```

### 1.2 Function Overloads for Flexibility

```typescript
// Single responsibility with overloads for convenience
interface ToolBuilder {
  register(tool: Tool): this;
  register(tools: Tool[], options: RegisterOptions): this;
  register(toolFactory: () => Tool[], options: RegisterOptions): this;
}

// Batch operations overload
async function executeTools(
  payload: Payload,
  tools: Tool[]
): Promise<Result>;
async function executeTools(
  payload: Payload,
  toolIds: string[],
  options: ExecutionOptions
): Promise<Result>;
async function executeTools(
  ...args: unknown[]
): Promise<Result> {
  // Implementation handles all overloads
}
```

---

## 2. Enhanced Error Message Format

**Specification:**
```
ERR[CODE] | [SEVERITY] | [CATEGORY]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Description: [Clear, concise explanation]
Context: [Relevant state/variables]
Remediation: [Specific steps to fix]
Documentation: [Link to docs section]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**Example Implementation (TypeScript):**
```typescript
class CognitiveError extends Error {
  constructor(
    readonly code: string,
    readonly severity: 'FATAL' | 'ERROR' | 'WARN',
    readonly category: string,
    readonly description: string,
    readonly remediation: string,
    readonly docLink: string
  ) {
    super();
    this.message = this.formatError();
  }

  private formatError(): string {
    return `ERR[${this.code}] | ${this.severity} | ${this.category}
Description: ${this.description}
Remediation: ${this.remediation}
Docs: https://xkernal.dev/docs/${this.docLink}`;
  }
}

// Usage
throw new CognitiveError(
  'TOOL_INIT_001',
  'ERROR',
  'Tool Initialization',
  'Tool "calculator" failed: missing required method sum()',
  'Implement sum() method in your Tool class',
  'tools/implementing-custom-tools#required-methods'
);
```

**C# Equivalent:**
```csharp
public class CognitiveException : Exception {
  public string Code { get; }
  public string Severity { get; }
  public string Remediation { get; }
  public string DocLink { get; }

  public CognitiveException(
    string code, string severity, string description,
    string remediation, string docLink
  ) : base(FormatError(code, severity, description, remediation, docLink)) {
    Code = code;
    Severity = severity;
    Remediation = remediation;
    DocLink = docLink;
  }

  private static string FormatError(
    string code, string severity, string description,
    string remediation, string docLink
  ) => $@"ERR[{code}] | {severity}
Description: {description}
Remediation: {remediation}
Docs: https://xkernal.dev/docs/{docLink}";
}
```

---

## 3. Comprehensive Examples

### 3.1 Hello World Example

**TypeScript:**
```typescript
import { createSubstrate, registerTools } from '@xkernal/sdk';

const main = async () => {
  // Create Substrate instance
  const substrate = await createSubstrate({
    name: 'HelloWorld',
    version: '1.0',
    timeout: 30000
  });

  // Register built-in tools
  await registerTools([
    { id: 'echo', execute: async (x) => x }
  ]);

  // Execute simple payload
  const result = await substrate.executePayload({
    toolId: 'echo',
    input: 'Hello, XKernal!'
  });

  console.log('Result:', result.output);
  process.exit(0);
};

main().catch(err => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
```

**C#:**
```csharp
using XKernal.SDK;

class Program {
  static async Task Main() {
    // Create Substrate instance
    var substrate = await CognitiveSubstrate.CreateAsync(
      new SubstrateConfig {
        Name = "HelloWorld",
        Version = "1.0",
        Timeout = TimeSpan.FromSeconds(30)
      }
    );

    // Register tools
    await substrate.RegisterToolsAsync(new[] {
      new Tool { Id = "echo", Handler = async (x) => x }
    });

    // Execute payload
    var result = await substrate.ExecutePayloadAsync(
      new Payload { ToolId = "echo", Input = "Hello, XKernal!" }
    );

    Console.WriteLine($"Result: {result.Output}");
  }
}
```

### 3.2 Memory Operations Example

```typescript
// Memory context management
const crew = await substrate.createCrew({
  name: 'analysis-crew',
  memory: {
    type: 'vector-store',
    config: { dimension: 768, indexType: 'hnsw' }
  }
});

// Add memory entries
await crew.memory.store({
  id: 'memory-001',
  embedding: vectorEmbedding,
  metadata: { type: 'conversation', timestamp: Date.now() }
});

// Query memory with timeout
const memories = await crew.memory.query(userQuery, {
  limit: 5,
  timeout: 5000, // NEW: explicit timeout handling
  threshold: 0.75
});
```

### 3.3 Error Handling Pattern

```typescript
async function safeExecute(payload: Payload): Promise<Result | null> {
  try {
    return await substrate.executePayload(payload);
  } catch (err) {
    if (err instanceof CognitiveError) {
      // Structured error with remediation
      logger.error({
        code: err.code,
        severity: err.severity,
        message: err.message
      });
      // Auto-remediation logic based on error code
      if (err.code === 'TOOL_TIMEOUT_001') {
        return await retryWithIncreasedTimeout(payload);
      }
    }
    return null;
  }
}
```

---

## 4. Missing Utility Functions (Week 27 Backlog)

### 4.1 Batch Operations

```typescript
interface BatchOptions {
  concurrency: number;
  timeout: number;
  failureMode: 'fast' | 'collect'; // Fail on first error vs collect all
}

async function executeBatch(
  payloads: Payload[],
  options: BatchOptions = { concurrency: 3, timeout: 30000, failureMode: 'collect' }
): Promise<BatchResult[]> {
  // Implementation with connection pooling
}

// Usage
const results = await substrate.executeBatch(payloads, {
  concurrency: 5,
  timeout: 60000,
  failureMode: 'collect'
});
```

### 4.2 Timeout Handling

```typescript
const withTimeout = async <T>(
  promise: Promise<T>,
  ms: number,
  errorCode = 'OPERATION_TIMEOUT'
): Promise<T> => {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) =>
      setTimeout(() => reject(
        new CognitiveError(errorCode, 'ERROR', 'Timeout', `Operation exceeded ${ms}ms`,
          'Reduce operation complexity or increase timeout', 'timeouts')
      ), ms)
    )
  ]);
};
```

---

## 5. Validation & Release Checklist

**Pre-Release v0.2 Validation:**
- [ ] TypeScript examples: all patterns tested on Node 18+
- [ ] C# examples: tested on .NET 7.0+
- [ ] Error messages: all error codes documented
- [ ] Performance: Hello World <5 min setup verified
- [ ] Adapter team patterns: validated against consumer interfaces
- [ ] Documentation: inline examples in all exported APIs
- [ ] Regression tests: Week 26-27 stability maintained

**Release Candidate Build:**
```bash
npm run build:sdk
npm run validate:examples
npm run test:e2e
npm publish --tag rc.2
```

---

## 6. Success Metrics

| Metric | Target | Current (Week 27) |
|--------|--------|------------------|
| Time-to-Hello-World | <5 min | ~10 min |
| TypeScript PASS Rate | 95%+ | 88% |
| C# PASS Rate | 90%+ | 81% (MARGINAL) |
| Example Coverage | 100% | 70% |
| Error Message Clarity | 100% adoption | 0% |

**Week 28 Deliverable:** SDK v0.2-RC1 with all improvements integrated and validated against Week 27 feedback.
