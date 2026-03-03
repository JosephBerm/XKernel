# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 06

## Phase: Phase 0

## Weekly Objective

Complete SDK project setup by integrating both TypeScript and C# SDKs into shared monorepo with Engineer 10. Establish development environment, CI/CD pipeline, and testing harness for stub validation.

## Document References

- **Primary:** Section 3.5.5 — TypeScript and C# SDKs; Section 6.1 — Phase 0
- **Supporting:** Engineer 10 monorepo deliverables; CI/CD best practices; npm/NuGet publishing

## Deliverables

- [ ] Integrate TypeScript and C# SDK projects into shared monorepo
- [ ] Set up CI/CD pipeline (lint, type-check, unit test, build, publish)
- [ ] Configure npm workspace (TypeScript SDK) and .NET solution (C# SDK)
- [ ] Create example project structure in both SDKs
- [ ] Document SDK development workflow and contribution guidelines
- [ ] Test build and packaging for both @cognitive-substrate/sdk and CognitiveSubstrate.SDK

## Technical Specifications

- CI/CD validates TypeScript compilation, ESLint, jest tests
- CI/CD validates C# compilation, StyleCop, xUnit tests
- Monorepo uses consistent versioning (CSCI v0.1 = SDKs v0.1.0)
- SDK stubs can be imported and instantiated without errors
- Documentation auto-generated from TypeScript interfaces (TypeDoc) and C# XML docs

## Dependencies

- **Blocked by:** Weeks 4-5
- **Blocking:** Week 7-8 (FFI binding layer)

## Acceptance Criteria

Monorepo fully operational; SDKs build and publish successfully; ready for FFI implementation

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

