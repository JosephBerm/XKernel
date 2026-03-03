# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 21

## Phase: Phase 2

## Weekly Objective

Package libcognitive v0.1 (5 reasoning patterns + error handling + crew utilities) for distribution via npm (@cognitive-substrate/libcognitive) and NuGet (CognitiveSubstrate.Libcognitive). Integrate into TypeScript and C# SDKs.

## Document References

- **Primary:** Section 3.5.2 — libcognitive: Standard Library; Section 6.3 — Phase 2
- **Supporting:** Phase 1 libcognitive implementations; CSCI v1.0; TypeScript and C# SDKs

## Deliverables

- [ ] Package libcognitive as npm module (@cognitive-substrate/libcognitive) with ReAct, CoT, Reflection, error handling, crew patterns
- [ ] Package libcognitive as NuGet module (CognitiveSubstrate.Libcognitive)
- [ ] Export pattern functions: ReAct, ChainOfThought, Reflection, retry, rollback, Supervisor, RoundRobin, Consensus
- [ ] Create pattern API documentation with examples
- [ ] Integrate libcognitive into TypeScript SDK (import { ReAct } from '@cognitive-substrate/libcognitive')
- [ ] Integrate libcognitive into C# SDK (using CognitiveSubstrate.Libcognitive;)
- [ ] Test end-to-end: SDK → libcognitive pattern → CSCI syscalls

## Technical Specifications

- libcognitive published with version v0.1.0 aligned with CSCI v1.0 and SDKs v0.1
- Patterns exported as composable functions: (config) => CT pattern definition
- Error handling strategies exposed: retry, rollback, escalate, degrade
- Crew utilities exposed: Supervisor, RoundRobin, Consensus, WorkerPool
- Package metadata includes dependencies (CSCI v1.0, SDK) and peer dependencies

## Dependencies

- **Blocked by:** Phase 1, Weeks 19-20
- **Blocking:** Week 22 (SDK polish); Phase 3 (SDKs v1.0)

## Acceptance Criteria

libcognitive v0.1 available on npm and NuGet; integrated into both SDKs

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

