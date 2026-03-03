# Engineer 9 — SDK: CSCI, libcognitive & SDKs — Week 05

## Phase: Phase 0

## Weekly Objective

Begin SDK stub generation based on CSCI v0.1. Create TypeScript (@cognitive-substrate/sdk) and C# (CognitiveSubstrate.SDK) project structures with interface stubs for all 22 syscalls. Coordinate monorepo setup with Engineer 10.

## Document References

- **Primary:** Section 3.5.5 — TypeScript and C# SDKs; Section 6.1 — Phase 0
- **Supporting:** CSCI v0.1 specification; Engineer 10 monorepo planning; npm/NuGet publishing strategy

## Deliverables

- [ ] Set up TypeScript SDK project structure (@cognitive-substrate/sdk)
- [ ] Generate TypeScript syscall binding interfaces for all 22 CSCI syscalls
- [ ] Set up C# SDK project structure (CognitiveSubstrate.SDK)
- [ ] Generate C# syscall binding interfaces for all 22 CSCI syscalls
- [ ] Establish npm and NuGet package metadata
- [ ] Coordinate with Engineer 10 on monorepo root structure and CI/CD

## Technical Specifications

- TypeScript SDK includes: tsconfig.json, package.json, src/syscalls/*.ts stub interfaces, src/index.ts
- C# SDK includes: .csproj, nuspec, src/Syscalls/*.cs stub interfaces, src/CognitiveSubstrate.cs
- Both SDKs include README.md with project status and roadmap
- SDK interfaces match CSCI v0.1 exactly (parameter names, return types, error codes)
- Monorepo uses workspace pattern (npm/yarn) or solution pattern (.NET)

## Dependencies

- **Blocked by:** Week 4
- **Blocking:** Week 6 (SDK setup completion); Week 7-8 (FFI implementation)

## Acceptance Criteria

TypeScript and C# SDK stubs complete; monorepo framework agreed with Engineer 10

## Design Principles Alignment

- **Cognitive-Native:** All syscall interfaces designed for tight integration with CT execution engine
- **Semantic Versioning:** CSCI follows major.minor.patch; SDKs track CSCI compatibility
- **Developer Experience:** TypeScript and C# SDKs provide strongly-typed, async-first APIs with IntelliSense
- **Interoperability:** CSCI syscalls are the unified contract; SDKs bridge language ecosystems
- **Testing:** Unit tests, integration tests with kernel team, and FFI layer validation
- **Documentation:** API docs, examples, tutorials for all public surface

