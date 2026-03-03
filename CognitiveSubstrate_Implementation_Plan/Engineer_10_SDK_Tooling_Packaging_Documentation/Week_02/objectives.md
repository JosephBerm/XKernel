# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 02

## Phase: 0 (Foundation & Monorepo Setup)

## Weekly Objective
Complete monorepo structure design. Define workspace boundaries, dependency policies, and tooling conventions. Create baseline directory structure and placeholder modules. Document design decisions for future contributors.

## Document References
- **Primary:** Section 6.1 — Phase 0, Week 5-6 (Monorepo, Bazel, CI/CD)
- **Supporting:** Section 3.5.3 (cs-pkg), Section 3.5.4 (Debugging Tools), Section 3.5.6 (Documentation Portal)

## Deliverables
- [ ] Complete monorepo structure with all Week_01 to Week_36 directories
- [ ] WORKSPACE configuration template
- [ ] Dependency policy document (what can depend on what)
- [ ] Module README templates for each layer
- [ ] DEVELOPMENT.md with contributing guidelines

## Technical Specifications
### Directory Structure Implementation
```
CognitiveSubstrate/
├── kernel/
│   ├── l0/           # L0: Core Rust runtime
│   └── BUILD         # Bazel target
├── services/
│   ├── BUILD         # L1 Rust services
│   └── ...
├── runtime/
│   ├── BUILD         # L2 Rust+TS runtime
│   └── ...
├── sdk/
│   ├── csci/         # CSCI library
│   ├── libcognitive/ # Cognitive core
│   ├── ts-sdk/       # TypeScript SDK
│   ├── cs-sdk/       # Multi-language SDK
│   ├── cs-pkg/       # Package manager
│   ├── tools/        # Debugging tools
│   ├── cs-ctl/       # CLI
│   └── BUILD
├── docs/             # Documentation
├── tests/            # Integration tests
├── benches/          # Benchmarks
└── BUILD
```

### Dependency Constraints
- L0 (kernel) → no dependencies on upper layers
- L1 (services) → may depend on L0
- L2 (runtime) → may depend on L0, L1
- L3 (SDK) → may depend on L0, L1, L2; internal SDK layering rules
- docs, tests, benches → depend on any SDK layer

## Dependencies
- **Blocked by:** Week 01 domain model review and architecture approval
- **Blocking:** Week 03-04 detailed monorepo implementation, all CI/CD setup

## Acceptance Criteria
- [ ] All directory structures created with placeholder BUILD files
- [ ] Dependency policy enforced by Bazel WORKSPACE visibility rules
- [ ] Module README templates guide future contributors
- [ ] DEVELOPMENT.md covers local setup, testing, build commands

## Design Principles Alignment
- **Cognitive-Native:** SDK layers match cognitive execution model layers
- **Isolation by Default:** Clear dependency boundaries prevent circular imports
- **Debuggability:** Tool components isolated, each with independent build targets
- **Open-Source Ready:** DEVELOPMENT.md enables external contributions
