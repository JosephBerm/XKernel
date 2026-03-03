# Week 21: libcognitive v0.1 Packaging & Distribution

**Project:** XKernal Cognitive Substrate OS
**Layer:** L3 SDK (Rust/TypeScript/C#)
**Phase:** Phase 2 (Abstraction Interfaces)
**Objective:** Package and distribute libcognitive v0.1 across npm (@cognitive/libcognitive) and NuGet (Cognitive.Libcognitive)
**Status:** In Progress
**Date:** 2026-03-02

---

## Executive Summary

Week 21 extends the Week 19-20 SDK implementations by packaging core reasoning patterns and utilities into production-ready distributions. This document specifies the export architecture, package structures, integration interfaces, and end-to-end validation pipelines for libcognitive across TypeScript and C# ecosystems.

**Key Deliverables:**
- 5 reasoning patterns (ReAct, ChainOfThought, Reflection, Supervisor, RoundRobin, Consensus)
- Error handling utilities (retry, rollback)
- npm package: `@cognitive/libcognitive@0.1.0`
- NuGet package: `Cognitive.Libcognitive@0.1.0`
- E2E test coverage (SDK → libcognitive → CSCI)

---

## 1. Architecture Overview

### 1.1 Package Hierarchy

```
XKernal Cognitive Substrate OS
├── libcognitive (Rust core)
│   ├── patterns/ (ReAct, CoT, Reflection, Supervisor, RoundRobin, Consensus)
│   ├── utilities/ (retry, rollback, error handling)
│   └── ffi/ (C-compatible interfaces)
├── SDKs
│   ├── typescript-sdk (npm: @cognitive/libcognitive)
│   │   ├── src/
│   │   │   ├── patterns/
│   │   │   ├── utilities/
│   │   │   ├── bindings/ (wasm + native)
│   │   │   └── index.ts
│   │   ├── package.json
│   │   └── tsconfig.json
│   └── csharp-sdk (NuGet: Cognitive.Libcognitive)
│       ├── src/
│       │   ├── Patterns/
│       │   ├── Utilities/
│       │   ├── Bindings/ (P/Invoke + native)
│       │   └── LibcognitiveSDK.cs
│       ├── Cognitive.Libcognitive.csproj
│       └── Cognitive.Libcognitive.nuspec
└── csci/ (CSCI layer integration)
```

### 1.2 Distribution Flow

```
libcognitive (Rust) [wasm-pack build]
    ↓
TypeScript pkg (dist/)  →  npm registry
    ↓                        ↓
SDK Layer            @cognitive/libcognitive@0.1
    ↓
Consumer Apps (Node.js, Web)

libcognitive (Rust) [cbindgen + compile to .so/.dll/.dylib]
    ↓
C# wrapper (generated bindings) → NuGet registry
    ↓                              ↓
SDK Layer                  Cognitive.Libcognitive@0.1
    ↓
Consumer Apps (.NET 6+)
```

---

## 2. Reasoning Patterns Export

### 2.1 Pattern Definitions

**Pattern Interface (shared across SDKs):**

```typescript
// packages/@cognitive/libcognitive/src/patterns/types.ts
export interface ReasoningPattern {
  name: string;
  version: string;
  execute(input: PatternInput): Promise<PatternOutput>;
  validate(input: PatternInput): ValidationResult;
}

export interface PatternInput {
  query: string;
  context?: Record<string, unknown>;
  constraints?: {
    maxSteps?: number;
    timeout?: number;
  };
}

export interface PatternOutput {
  reasoning: string[];
  finalAnswer: string;
  confidence: number;
  metadata: {
    executionTime: number;
    stepsExecuted: number;
    patternsUsed: string[];
  };
}

export interface ValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}
```

### 2.2 ReAct Pattern

**TypeScript Implementation:**

```typescript
// packages/@cognitive/libcognitive/src/patterns/react.ts
import { ReasoningPattern, PatternInput, PatternOutput } from './types';

export class ReActPattern implements ReasoningPattern {
  name = 'ReAct';
  version = '0.1.0';

  async execute(input: PatternInput): Promise<PatternOutput> {
    const reasoning: string[] = [];
    const maxSteps = input.constraints?.maxSteps ?? 10;
    let step = 0;

    reasoning.push(`[THOUGHT] Analyzing query: "${input.query}"`);

    while (step < maxSteps) {
      const action = await this.selectAction(input, reasoning);
      if (!action) break;

      reasoning.push(`[ACTION] ${action.type}: ${action.description}`);

      const observation = await this.executeAction(action);
      reasoning.push(`[OBSERVATION] ${observation}`);

      if (this.isTerminal(observation)) {
        break;
      }

      step++;
    }

    const finalAnswer = this.extractAnswer(reasoning);
    return {
      reasoning,
      finalAnswer,
      confidence: 0.85,
      metadata: {
        executionTime: Date.now(),
        stepsExecuted: step,
        patternsUsed: ['react'],
      },
    };
  }

  validate(input: PatternInput): { valid: boolean; errors: string[] } {
    const errors: string[] = [];
    if (!input.query || input.query.trim().length === 0) {
      errors.push('Query cannot be empty');
    }
    if (input.constraints?.maxSteps && input.constraints.maxSteps < 1) {
      errors.push('maxSteps must be >= 1');
    }
    return { valid: errors.length === 0, errors };
  }

  private async selectAction(
    input: PatternInput,
    reasoning: string[]
  ): Promise<{ type: string; description: string } | null> {
    // Delegate to CSCI LLM layer for action selection
    const actionModule = await import('../../../csci');
    return actionModule.selectNextAction(input.query, reasoning);
  }

  private async executeAction(action: {
    type: string;
    description: string;
  }): Promise<string> {
    // Execute against CSCI tools/search layer
    const csci = await import('../../../csci');
    return csci.executeTool(action.type, action.description);
  }

  private isTerminal(observation: string): boolean {
    return observation.includes('[FINAL_ANSWER]');
  }

  private extractAnswer(reasoning: string[]): string {
    const finalLine = reasoning.find((line) => line.includes('[FINAL_ANSWER]'));
    return finalLine ? finalLine.replace('[FINAL_ANSWER]', '').trim() : '';
  }
}
```

**C# Implementation:**

```csharp
// Cognitive.Libcognitive/src/Patterns/ReActPattern.cs
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace Cognitive.Libcognitive.Patterns
{
    public class ReActPattern : IReasoningPattern
    {
        public string Name => "ReAct";
        public string Version => "0.1.0";

        public async Task<PatternOutput> ExecuteAsync(PatternInput input)
        {
            var reasoning = new List<string>();
            var maxSteps = input.Constraints?.MaxSteps ?? 10;
            int step = 0;

            reasoning.Add($"[THOUGHT] Analyzing query: \"{input.Query}\"");

            while (step < maxSteps)
            {
                var action = await SelectActionAsync(input, reasoning);
                if (action == null) break;

                reasoning.Add($"[ACTION] {action.Type}: {action.Description}");

                var observation = await ExecuteActionAsync(action);
                reasoning.Add($"[OBSERVATION] {observation}");

                if (IsTerminal(observation))
                {
                    break;
                }

                step++;
            }

            var finalAnswer = ExtractAnswer(reasoning);
            return new PatternOutput
            {
                Reasoning = reasoning,
                FinalAnswer = finalAnswer,
                Confidence = 0.85,
                Metadata = new OutputMetadata
                {
                    ExecutionTime = DateTime.UtcNow.Millisecond,
                    StepsExecuted = step,
                    PatternsUsed = new[] { "react" },
                },
            };
        }

        public ValidationResult Validate(PatternInput input)
        {
            var errors = new List<string>();

            if (string.IsNullOrWhiteSpace(input.Query))
            {
                errors.Add("Query cannot be empty");
            }

            if (input.Constraints?.MaxSteps.HasValue == true && input.Constraints.MaxSteps < 1)
            {
                errors.Add("MaxSteps must be >= 1");
            }

            return new ValidationResult
            {
                Valid = errors.Count == 0,
                Errors = errors,
            };
        }

        private async Task<ActionDescriptor> SelectActionAsync(
            PatternInput input,
            List<string> reasoning)
        {
            // P/Invoke call to native libcognitive.so
            var csciModule = await Task.FromResult(
                new LibcognitiveInterop()
            );
            return csciModule.SelectNextAction(input.Query, reasoning);
        }

        private async Task<string> ExecuteActionAsync(ActionDescriptor action)
        {
            var csci = await Task.FromResult(
                new LibcognitiveInterop()
            );
            return csci.ExecuteTool(action.Type, action.Description);
        }

        private bool IsTerminal(string observation)
        {
            return observation.Contains("[FINAL_ANSWER]");
        }

        private string ExtractAnswer(List<string> reasoning)
        {
            var finalLine = reasoning.Find(line => line.Contains("[FINAL_ANSWER]"));
            return finalLine != null
                ? finalLine.Replace("[FINAL_ANSWER]", "").Trim()
                : "";
        }
    }
}
```

### 2.3 Chain of Thought Pattern

**TypeScript:**

```typescript
// packages/@cognitive/libcognitive/src/patterns/cot.ts
export class ChainOfThoughtPattern implements ReasoningPattern {
  name = 'ChainOfThought';
  version = '0.1.0';

  async execute(input: PatternInput): Promise<PatternOutput> {
    const reasoning: string[] = [];
    reasoning.push(`[STEP 1] Understanding: ${input.query}`);

    const csci = await import('../../../csci');
    const decomposition = await csci.decomposeProblem(input.query);

    for (let i = 0; i < decomposition.steps.length; i++) {
      reasoning.push(`[STEP ${i + 2}] ${decomposition.steps[i]}`);
    }

    const finalAnswer = decomposition.synthesis;
    reasoning.push(`[SYNTHESIS] ${finalAnswer}`);

    return {
      reasoning,
      finalAnswer,
      confidence: 0.88,
      metadata: {
        executionTime: Date.now(),
        stepsExecuted: decomposition.steps.length,
        patternsUsed: ['cot'],
      },
    };
  }

  validate(input: PatternInput) {
    return { valid: !!input.query, errors: [] };
  }
}
```

### 2.4 Reflection Pattern

**TypeScript:**

```typescript
// packages/@cognitive/libcognitive/src/patterns/reflection.ts
export class ReflectionPattern implements ReasoningPattern {
  name = 'Reflection';
  version = '0.1.0';

  async execute(input: PatternInput): Promise<PatternOutput> {
    const reasoning: string[] = [];

    reasoning.push(`[INITIAL_RESPONSE] Processing: ${input.query}`);

    const csci = await import('../../../csci');
    const initial = await csci.generateInitialResponse(input.query);
    reasoning.push(`Initial thought: ${initial}`);

    reasoning.push('[CRITIQUE] Self-evaluating response...');
    const critique = await csci.critiqueSolution(initial);
    reasoning.push(`Critique: ${critique}`);

    if (critique.requiresRevision) {
      reasoning.push('[REVISION] Improving response...');
      const revised = await csci.reviseResponse(initial, critique);
      reasoning.push(`Revised: ${revised}`);
      return {
        reasoning,
        finalAnswer: revised,
        confidence: 0.82,
        metadata: {
          executionTime: Date.now(),
          stepsExecuted: 3,
          patternsUsed: ['reflection'],
        },
      };
    }

    return {
      reasoning,
      finalAnswer: initial,
      confidence: 0.90,
      metadata: {
        executionTime: Date.now(),
        stepsExecuted: 2,
        patternsUsed: ['reflection'],
      },
    };
  }

  validate(input: PatternInput) {
    return { valid: !!input.query, errors: [] };
  }
}
```

### 2.5 Supervisor Pattern

**TypeScript:**

```typescript
// packages/@cognitive/libcognitive/src/patterns/supervisor.ts
export class SupervisorPattern implements ReasoningPattern {
  name = 'Supervisor';
  version = '0.1.0';

  async execute(input: PatternInput): Promise<PatternOutput> {
    const reasoning: string[] = [];
    reasoning.push(`[SUPERVISOR_INIT] Delegating: ${input.query}`);

    const csci = await import('../../../csci');
    const agents = ['analyzer', 'planner', 'executor'];
    const results: Record<string, string> = {};

    for (const agent of agents) {
      reasoning.push(`[DELEGATE] Assigning to ${agent}...`);
      results[agent] = await csci.delegateToAgent(agent, input.query);
      reasoning.push(`[RESULT_${agent.toUpperCase()}] ${results[agent]}`);
    }

    reasoning.push('[COORDINATION] Synthesizing results...');
    const synthesis = await csci.synthesizeResults(results);
    reasoning.push(`Final synthesis: ${synthesis}`);

    return {
      reasoning,
      finalAnswer: synthesis,
      confidence: 0.89,
      metadata: {
        executionTime: Date.now(),
        stepsExecuted: agents.length + 1,
        patternsUsed: ['supervisor'],
      },
    };
  }

  validate(input: PatternInput) {
    return { valid: !!input.query, errors: [] };
  }
}
```

### 2.6 RoundRobin & Consensus Patterns

**TypeScript:**

```typescript
// packages/@cognitive/libcognitive/src/patterns/roundrobin.ts
export class RoundRobinPattern implements ReasoningPattern {
  name = 'RoundRobin';
  version = '0.1.0';

  async execute(input: PatternInput): Promise<PatternOutput> {
    const reasoning: string[] = [];
    const csci = await import('../../../csci');
    const perspectives = await csci.getRoundRobinPerspectives(5);

    for (let i = 0; i < perspectives.length; i++) {
      reasoning.push(`[PERSPECTIVE_${i + 1}] ${perspectives[i].viewpoint}`);
      const analysis = await csci.analyzeFromPerspective(
        input.query,
        perspectives[i]
      );
      reasoning.push(`Analysis: ${analysis}`);
    }

    return {
      reasoning,
      finalAnswer: reasoning.join(' | '),
      confidence: 0.85,
      metadata: {
        executionTime: Date.now(),
        stepsExecuted: perspectives.length,
        patternsUsed: ['roundrobin'],
      },
    };
  }

  validate(input: PatternInput) {
    return { valid: !!input.query, errors: [] };
  }
}

// packages/@cognitive/libcognitive/src/patterns/consensus.ts
export class ConsensusPattern implements ReasoningPattern {
  name = 'Consensus';
  version = '0.1.0';

  async execute(input: PatternInput): Promise<PatternOutput> {
    const reasoning: string[] = [];
    const csci = await import('../../../csci');

    reasoning.push(`[CONSENSUS_INIT] Gathering votes: ${input.query}`);

    const votes = await csci.gatherVotes(input.query, 5);
    reasoning.push(`Votes received: ${votes.length}`);

    const consensus = this.calculateConsensus(votes);
    reasoning.push(`[CONSENSUS_RESULT] ${JSON.stringify(consensus)}`);

    return {
      reasoning,
      finalAnswer: consensus.agreedAnswer,
      confidence: consensus.confidence,
      metadata: {
        executionTime: Date.now(),
        stepsExecuted: 2,
        patternsUsed: ['consensus'],
      },
    };
  }

  private calculateConsensus(votes: string[]): {
    agreedAnswer: string;
    confidence: number;
  } {
    const counts = new Map<string, number>();
    votes.forEach((vote) => {
      counts.set(vote, (counts.get(vote) || 0) + 1);
    });

    const sorted = Array.from(counts.entries()).sort((a, b) => b[1] - a[1]);
    const [answer, count] = sorted[0];

    return {
      agreedAnswer: answer,
      confidence: count / votes.length,
    };
  }

  validate(input: PatternInput) {
    return { valid: !!input.query, errors: [] };
  }
}
```

---

## 3. Error Handling & Utilities

### 3.1 Retry Utility

**TypeScript:**

```typescript
// packages/@cognitive/libcognitive/src/utilities/retry.ts
export interface RetryConfig {
  maxAttempts: number;
  backoffMultiplier: number;
  initialDelayMs: number;
  maxDelayMs: number;
}

export async function withRetry<T>(
  fn: () => Promise<T>,
  config: Partial<RetryConfig> = {}
): Promise<T> {
  const defaultConfig: RetryConfig = {
    maxAttempts: 3,
    backoffMultiplier: 2,
    initialDelayMs: 100,
    maxDelayMs: 5000,
  };

  const finalConfig = { ...defaultConfig, ...config };
  let lastError: Error | null = null;
  let delayMs = finalConfig.initialDelayMs;

  for (let attempt = 1; attempt <= finalConfig.maxAttempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error as Error;

      if (attempt === finalConfig.maxAttempts) break;

      const actualDelay = Math.min(
        delayMs,
        finalConfig.maxDelayMs
      );
      await new Promise((resolve) => setTimeout(resolve, actualDelay));

      delayMs *= finalConfig.backoffMultiplier;
    }
  }

  throw lastError || new Error('Max retries exceeded');
}
```

**C#:**

```csharp
// Cognitive.Libcognitive/src/Utilities/RetryHelper.cs
using System;
using System.Threading.Tasks;

namespace Cognitive.Libcognitive.Utilities
{
    public class RetryConfig
    {
        public int MaxAttempts { get; set; } = 3;
        public double BackoffMultiplier { get; set; } = 2.0;
        public int InitialDelayMs { get; set; } = 100;
        public int MaxDelayMs { get; set; } = 5000;
    }

    public static class RetryHelper
    {
        public static async Task<T> WithRetryAsync<T>(
            Func<Task<T>> fn,
            RetryConfig config = null)
        {
            config = config ?? new RetryConfig();
            Exception lastException = null;
            int delayMs = config.InitialDelayMs;

            for (int attempt = 1; attempt <= config.MaxAttempts; attempt++)
            {
                try
                {
                    return await fn();
                }
                catch (Exception ex)
                {
                    lastException = ex;

                    if (attempt == config.MaxAttempts)
                        break;

                    int actualDelay = Math.Min(delayMs, config.MaxDelayMs);
                    await Task.Delay(actualDelay);

                    delayMs = (int)(delayMs * config.BackoffMultiplier);
                }
            }

            throw lastException ?? new Exception("Max retries exceeded");
        }
    }
}
```

### 3.2 Rollback Utility

**TypeScript:**

```typescript
// packages/@cognitive/libcognitive/src/utilities/rollback.ts
export interface Checkpoint {
  id: string;
  state: Record<string, unknown>;
  timestamp: number;
}

export class RollbackManager {
  private checkpoints: Map<string, Checkpoint> = new Map();

  saveCheckpoint(id: string, state: Record<string, unknown>): Checkpoint {
    const checkpoint: Checkpoint = {
      id,
      state: structuredClone(state),
      timestamp: Date.now(),
    };
    this.checkpoints.set(id, checkpoint);
    return checkpoint;
  }

  restore(id: string): Record<string, unknown> {
    const checkpoint = this.checkpoints.get(id);
    if (!checkpoint) {
      throw new Error(`Checkpoint "${id}" not found`);
    }
    return structuredClone(checkpoint.state);
  }

  rollback(id: string, targetId: string): void {
    const target = this.checkpoints.get(targetId);
    if (!target) {
      throw new Error(`Target checkpoint "${targetId}" not found`);
    }
    this.checkpoints.set(id, { ...target, id });
  }

  listCheckpoints(): Checkpoint[] {
    return Array.from(this.checkpoints.values());
  }

  deleteCheckpoint(id: string): boolean {
    return this.checkpoints.delete(id);
  }
}
```

---

## 4. npm Package Structure

### 4.1 package.json

```json
{
  "name": "@cognitive/libcognitive",
  "version": "0.1.0",
  "description": "XKernal libcognitive SDK for reasoning patterns and utilities",
  "main": "dist/index.js",
  "module": "dist/index.esm.js",
  "types": "dist/index.d.ts",
  "files": [
    "dist",
    "README.md",
    "LICENSE"
  ],
  "scripts": {
    "build": "tsc && rollup -c",
    "test": "jest --coverage",
    "test:e2e": "jest --config jest.e2e.config.js",
    "lint": "eslint src/**/*.ts",
    "prepublish": "npm run build && npm run test"
  },
  "exports": {
    ".": {
      "import": "./dist/index.esm.js",
      "require": "./dist/index.js",
      "types": "./dist/index.d.ts"
    },
    "./patterns": {
      "import": "./dist/patterns/index.esm.js",
      "require": "./dist/patterns/index.js",
      "types": "./dist/patterns/index.d.ts"
    },
    "./utilities": {
      "import": "./dist/utilities/index.esm.js",
      "require": "./dist/utilities/index.js",
      "types": "./dist/utilities/index.d.ts"
    }
  },
  "dependencies": {
    "tslib": "^2.6.0"
  },
  "devDependencies": {
    "@types/jest": "^29.5.0",
    "@typescript-eslint/eslint-plugin": "^6.0.0",
    "jest": "^29.5.0",
    "rollup": "^3.25.0",
    "typescript": "^5.1.0"
  },
  "keywords": [
    "cognitive",
    "reasoning",
    "patterns",
    "llm",
    "xkernal"
  ],
  "license": "Apache-2.0",
  "repository": {
    "type": "git",
    "url": "https://github.com/xkernal/cognitive-sdk.git"
  }
}
```

### 4.2 Export Index

```typescript
// packages/@cognitive/libcognitive/src/index.ts
export {
  ReActPattern,
  ChainOfThoughtPattern,
  ReflectionPattern,
  SupervisorPattern,
  RoundRobinPattern,
  ConsensusPattern,
} from './patterns';

export {
  withRetry,
  RollbackManager,
} from './utilities';

export type {
  ReasoningPattern,
  PatternInput,
  PatternOutput,
  ValidationResult,
} from './patterns/types';

export const VERSION = '0.1.0';
export const PATTERNS = [
  'ReAct',
  'ChainOfThought',
  'Reflection',
  'Supervisor',
  'RoundRobin',
  'Consensus',
];
```

---

## 5. NuGet Package Structure

### 5.1 Project File (.csproj)

```xml
<!-- Cognitive.Libcognitive.csproj -->
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net6.0;net7.0;net8.0</TargetFramework>
    <Version>0.1.0</Version>
    <Authors>XKernal Team</Authors>
    <Description>XKernal libcognitive SDK for reasoning patterns</Description>
    <PackageId>Cognitive.Libcognitive</PackageId>
    <PackageTags>cognitive;reasoning;patterns;llm;xkernal</PackageTags>
    <RepositoryUrl>https://github.com/xkernal/cognitive-sdk</RepositoryUrl>
    <LangVersion>latest</LangVersion>
    <GeneratePackageOnBuild>true</GeneratePackageOnBuild>
    <PackageReleaseNotes>
      v0.1.0: Initial release with ReAct, CoT, Reflection, Supervisor, RoundRobin, Consensus patterns
    </PackageReleaseNotes>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="System.Runtime.InteropServices" Version="4.3.0" />
  </ItemGroup>

  <ItemGroup>
    <EmbeddedResource Include="runtimes/**/*.so" />
    <EmbeddedResource Include="runtimes/**/*.dll" />
    <EmbeddedResource Include="runtimes/**/*.dylib" />
  </ItemGroup>
</Project>
```

### 5.2 C# Pattern Registry

```csharp
// Cognitive.Libcognitive/src/PatternRegistry.cs
using System;
using System.Collections.Generic;
using Cognitive.Libcognitive.Patterns;

namespace Cognitive.Libcognitive
{
    public static class PatternRegistry
    {
        private static readonly Dictionary<string, Type> Patterns =
            new()
            {
                { "ReAct", typeof(ReActPattern) },
                { "ChainOfThought", typeof(ChainOfThoughtPattern) },
                { "Reflection", typeof(ReflectionPattern) },
                { "Supervisor", typeof(SupervisorPattern) },
                { "RoundRobin", typeof(RoundRobinPattern) },
                { "Consensus", typeof(ConsensusPattern) },
            };

        public static IReasoningPattern CreatePattern(string name)
        {
            if (!Patterns.TryGetValue(name, out var type))
                throw new ArgumentException($"Unknown pattern: {name}");

            return (IReasoningPattern)Activator.CreateInstance(type);
        }

        public static IEnumerable<string> GetAvailablePatterns() => Patterns.Keys;

        public static string Version => "0.1.0";
    }
}
```

---

## 6. End-to-End Testing

### 6.1 TypeScript E2E Test

```typescript
// packages/@cognitive/libcognitive/__tests__/e2e.test.ts
import {
  ReActPattern,
  ChainOfThoughtPattern,
  SupervisorPattern,
  ConsensusPattern,
  withRetry,
} from '../src/index';

describe('E2E: SDK → libcognitive → CSCI', () => {
  it('should execute ReAct pattern end-to-end', async () => {
    const pattern = new ReActPattern();
    const result = await pattern.execute({
      query: 'What is the capital of France?',
      constraints: { maxSteps: 5 },
    });

    expect(result.finalAnswer).toBeTruthy();
    expect(result.reasoning.length).toBeGreaterThan(0);
    expect(result.metadata.patternsUsed).toContain('react');
  });

  it('should execute ChainOfThought pattern', async () => {
    const pattern = new ChainOfThoughtPattern();
    const result = await pattern.execute({
      query: 'Solve: 2+2=?',
    });

    expect(result.confidence).toBeGreaterThan(0.8);
    expect(result.metadata.stepsExecuted).toBeGreaterThan(0);
  });

  it('should support pattern composition via Supervisor', async () => {
    const pattern = new SupervisorPattern();
    const result = await pattern.execute({
      query: 'Analyze the impact of AI on healthcare',
    });

    expect(result.metadata.stepsExecuted).toBeGreaterThanOrEqual(2);
  });

  it('should handle consensus across multiple votes', async () => {
    const pattern = new ConsensusPattern();
    const result = await pattern.execute({
      query: 'Best programming language for systems?',
    });

    expect(result.confidence).toBeGreaterThanOrEqual(0.5);
    expect(result.confidence).toBeLessThanOrEqual(1.0);
  });

  it('should retry on transient failures', async () => {
    let attempts = 0;
    const fn = async () => {
      attempts++;
      if (attempts < 2) throw new Error('Transient failure');
      return 'success';
    };

    const result = await withRetry(fn, {
      maxAttempts: 3,
      initialDelayMs: 10,
    });

    expect(result).toBe('success');
    expect(attempts).toBe(2);
  });

  it('validates pattern input correctly', () => {
    const pattern = new ReActPattern();
    const validation = pattern.validate({ query: '' });

    expect(validation.valid).toBe(false);
    expect(validation.errors.length).toBeGreaterThan(0);
  });

  it('integrates with CSCI layer for tool execution', async () => {
    const pattern = new ReActPattern();
    const result = await pattern.execute({
      query: 'Current weather in New York',
      context: { location: 'New York' },
    });

    expect(result.metadata.patternsUsed).toContain('react');
  });
});
```

### 6.2 C# E2E Test

```csharp
// Cognitive.Libcognitive.Tests/E2ETests.cs
using System;
using System.Threading.Tasks;
using Xunit;
using Cognitive.Libcognitive;
using Cognitive.Libcognitive.Patterns;
using Cognitive.Libcognitive.Utilities;

namespace Cognitive.Libcognitive.Tests
{
    public class E2ETests
    {
        [Fact]
        public async Task ExecuteReActPatternEndToEnd()
        {
            var pattern = new ReActPattern();
            var input = new PatternInput
            {
                Query = "What is the capital of France?",
                Constraints = new ConstraintConfig { MaxSteps = 5 },
            };

            var result = await pattern.ExecuteAsync(input);

            Assert.NotNull(result.FinalAnswer);
            Assert.NotEmpty(result.Reasoning);
            Assert.Contains("react", result.Metadata.PatternsUsed);
        }

        [Fact]
        public async Task ExecuteChainOfThoughtPattern()
        {
            var pattern = new ChainOfThoughtPattern();
            var result = await pattern.ExecuteAsync(
                new PatternInput { Query = "Solve: 2+2=?" }
            );

            Assert.True(result.Confidence > 0.8);
            Assert.True(result.Metadata.StepsExecuted > 0);
        }

        [Fact]
        public async Task SupportPatternCompositionViaSupervisor()
        {
            var pattern = new SupervisorPattern();
            var result = await pattern.ExecuteAsync(
                new PatternInput
                {
                    Query = "Analyze the impact of AI on healthcare",
                }
            );

            Assert.True(result.Metadata.StepsExecuted >= 2);
        }

        [Fact]
        public async Task HandleConsensusAcrossVotes()
        {
            var pattern = new ConsensusPattern();
            var result = await pattern.ExecuteAsync(
                new PatternInput
                {
                    Query = "Best programming language?",
                }
            );

            Assert.True(result.Confidence >= 0.5 && result.Confidence <= 1.0);
        }

        [Fact]
        public async Task RetryOnTransientFailures()
        {
            int attempts = 0;
            Func<Task<string>> fn = async () =>
            {
                attempts++;
                if (attempts < 2)
                    throw new InvalidOperationException("Transient failure");
                return await Task.FromResult("success");
            };

            var result = await RetryHelper.WithRetryAsync(fn, new RetryConfig
            {
                MaxAttempts = 3,
                InitialDelayMs = 10,
            });

            Assert.Equal("success", result);
            Assert.Equal(2, attempts);
        }

        [Fact]
        public void ValidatePatternInput()
        {
            var pattern = new ReActPattern();
            var validation = pattern.Validate(new PatternInput { Query = "" });

            Assert.False(validation.Valid);
            Assert.NotEmpty(validation.Errors);
        }

        [Fact]
        public void ListAvailablePatterns()
        {
            var patterns = PatternRegistry.GetAvailablePatterns();

            Assert.Contains("ReAct", patterns);
            Assert.Contains("ChainOfThought", patterns);
            Assert.Contains("Reflection", patterns);
            Assert.Contains("Supervisor", patterns);
            Assert.Contains("RoundRobin", patterns);
            Assert.Contains("Consensus", patterns);
        }
    }
}
```

---

## 7. Integration with CSCI Layer

### 7.1 TypeScript CSCI Bridge

```typescript
// packages/@cognitive/libcognitive/src/csci-bridge.ts
export interface CSCIBridge {
  selectNextAction(query: string, history: string[]): Promise<Action>;
  executeTool(toolType: string, description: string): Promise<string>;
  decomposeProblem(query: string): Promise<DecompositionResult>;
  generateInitialResponse(query: string): Promise<string>;
  critiqueSolution(response: string): Promise<CritiqueResult>;
  reviseResponse(initial: string, critique: CritiqueResult): Promise<string>;
  delegateToAgent(agent: string, task: string): Promise<string>;
  synthesizeResults(results: Record<string, string>): Promise<string>;
  getRoundRobinPerspectives(count: number): Promise<Perspective[]>;
  analyzeFromPerspective(query: string, perspective: Perspective): Promise<string>;
  gatherVotes(query: string, voterCount: number): Promise<string[]>;
}

export interface Action {
  type: string;
  description: string;
}

export interface DecompositionResult {
  steps: string[];
  synthesis: string;
}

export interface CritiqueResult {
  evaluation: string;
  requiresRevision: boolean;
}

export interface Perspective {
  viewpoint: string;
  expertise: string;
}
```

### 7.2 C# CSCI Interop

```csharp
// Cognitive.Libcognitive/src/Bindings/LibcognitiveInterop.cs
using System;
using System.Runtime.InteropServices;
using System.Collections.Generic;

namespace Cognitive.Libcognitive.Bindings
{
    public class LibcognitiveInterop
    {
        private const string LibName = "libcognitive";

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr libcog_select_action(
            string query,
            string history
        );

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr libcog_execute_tool(
            string toolType,
            string description
        );

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern IntPtr libcog_decompose_problem(string query);

        [DllImport(LibName, CallingConvention = CallingConvention.Cdecl)]
        private static extern void libcog_free(IntPtr ptr);

        public ActionDescriptor SelectNextAction(string query, List<string> history)
        {
            string historyJson = string.Join("|", history);
            IntPtr result = libcog_select_action(query, historyJson);
            string resultStr = Marshal.PtrToStringAnsi(result);
            libcog_free(result);

            return ParseActionDescriptor(resultStr);
        }

        public string ExecuteTool(string toolType, string description)
        {
            IntPtr result = libcog_execute_tool(toolType, description);
            string resultStr = Marshal.PtrToStringAnsi(result);
            libcog_free(result);
            return resultStr;
        }

        public DecompositionResult DecomposeProblem(string query)
        {
            IntPtr result = libcog_decompose_problem(query);
            string resultStr = Marshal.PtrToStringAnsi(result);
            libcog_free(result);

            return ParseDecompositionResult(resultStr);
        }

        private ActionDescriptor ParseActionDescriptor(string json)
        {
            // Parse JSON response
            return new ActionDescriptor { Type = "search", Description = json };
        }

        private DecompositionResult ParseDecompositionResult(string json)
        {
            return new DecompositionResult
            {
                Steps = new List<string> { json },
                Synthesis = json,
            };
        }
    }

    public class ActionDescriptor
    {
        public string Type { get; set; }
        public string Description { get; set; }
    }

    public class DecompositionResult
    {
        public List<string> Steps { get; set; }
        public string Synthesis { get; set; }
    }
}
```

---

## 8. Release & Distribution

### 8.1 npm Publishing

```bash
# Build and publish to npm
npm run prepublish
npm publish --access public

# Verify publication
npm view @cognitive/libcognitive@0.1.0

# Install in consumer project
npm install @cognitive/libcognitive@0.1.0
```

### 8.2 NuGet Publishing

```bash
# Package creation
dotnet pack Cognitive.Libcognitive.csproj -c Release -o ./nuget

# Push to NuGet.org
dotnet nuget push nuget/Cognitive.Libcognitive.0.1.0.nupkg \
  --api-key $NUGET_API_KEY \
  --source https://api.nuget.org/v3/index.json

# Verify publication
dotnet package search Cognitive.Libcognitive

# Install in consumer project
dotnet add package Cognitive.Libcognitive --version 0.1.0
```

---

## 9. Success Criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| npm package publication | @cognitive/libcognitive@0.1.0 live | Pending |
| NuGet package publication | Cognitive.Libcognitive@0.1.0 live | Pending |
| Pattern exports (count) | 6 (ReAct, CoT, Reflection, Supervisor, RR, Consensus) | In Progress |
| Utility exports (count) | 2 (retry, rollback) | In Progress |
| E2E test coverage | ≥80% | In Progress |
| SDK integration (TS) | 22+ bindings → libcognitive | On Track |
| SDK integration (C#) | 22+ bindings → libcognitive | On Track |
| Documentation | README, API docs, examples | Pending |

---

## 10. Timeline & Dependencies

**Week 21 (Current):**
- Finalize pattern implementations (ReAct, CoT, Reflection, Supervisor, RoundRobin, Consensus)
- Package structure finalization (npm & NuGet)
- E2E test implementation and validation
- Package publication (npm & NuGet)

**Week 22 (Next):**
- CSCI layer integration validation
- Performance benchmarking
- Documentation & examples
- Consumer adoption pilot

**Dependencies:**
- libcognitive Rust core (Week 18-19) ✓
- TypeScript SDK v0.1 (Week 19) ✓
- C# SDK v0.1 (Week 20) ✓
- CSCI layer integration points (Week 21)

---

## 11. References & Related Documents

- **Week 19:** TypeScript SDK v0.1 Specification
- **Week 20:** C# SDK v0.1 & P/Invoke Bindings
- **libcognitive:** Core Rust Implementation
- **CSCI:** Cognitive Substrate Computation Interface
- npm Registry: https://npmjs.com/@cognitive/libcognitive
- NuGet Registry: https://nuget.org/packages/Cognitive.Libcognitive

---

**Document Status:** Draft
**Last Updated:** 2026-03-02
**Next Review:** Week 22 Planning
