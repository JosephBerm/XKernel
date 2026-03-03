# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 33

## Phase: 3 (Cloud Deployment, Documentation & Launch)

## Weekly Objective
Prepare open-source repository for launch. Add Apache 2.0 license headers to all files. Create CONTRIBUTING.md guide. Structure public repository. Build README with quick-start instructions.

## Document References
- **Primary:** Section 6.4 — Phase 3, Week 33-34 (Open-source repo preparation)
- **Supporting:** Section 6.4 — Week 34-36 (Publish benchmarks, launch)

## Deliverables
- [ ] Apache 2.0 license headers on all source files
- [ ] LICENSE file in repository root
- [ ] CONTRIBUTING.md with development setup and contribution guidelines
- [ ] Public repository structure and README
- [ ] Code of Conduct (Contributor Covenant)
- [ ] SECURITY.md for responsible vulnerability disclosure
- [ ] GOVERNANCE.md defining decision-making process
- [ ] Issue and PR templates for community contributions
- [ ] Repository tagging and release process documentation

## Technical Specifications
### Apache 2.0 License Header Template
```rust
// Copyright 2026 Cognitive Substrate Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Module documentation...
```

### CONTRIBUTING.md Structure
```markdown
# Contributing to Cognitive Substrate

Thank you for your interest in contributing to Cognitive Substrate!

## Getting Started

### Development Environment Setup
```bash
# Clone repository
git clone https://github.com/cognitive-substrate/cognitive-substrate.git
cd cognitive-substrate

# Install dependencies
./scripts/setup_dev_environment.sh

# Build
bazel build //...

# Run tests
bazel test //...
```

### Code Style
- Rust: Follow standard Rust conventions (cargo fmt, cargo clippy)
- TypeScript: ESLint with provided configuration
- Documentation: Markdown with proper cross-references

### Commit Message Format
```
<type>(<scope>): <subject>

<body>

<footer>
```

Examples:
- `feat(cs-trace): add filtering by syscall type`
- `fix(cs-profile): correct memory accounting bug`
- `docs(getting-started): clarify first-time setup`

### Pull Request Process
1. Fork the repository
2. Create feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Submit PR with description of changes
5. Address review comments
6. Squash commits if requested

### Running CI/CD Locally
```bash
./run_local_ci.sh          # Full CI/CD pipeline
./run_local_ci.sh --quick  # Quick feedback loop
```

### Testing
- Unit tests: `bazel test //sdk/...`
- Integration tests: `bazel test //tests/...`
- Minimum coverage: 80%

## Reporting Bugs

Use GitHub Issues with:
- **Title:** Clear, concise description
- **Environment:** OS, Cognitive Substrate version, hardware
- **Steps to reproduce:** Exact steps to trigger bug
- **Expected behavior:** What should happen
- **Actual behavior:** What actually happens
- **Attachments:** Screenshots, logs, core dumps if applicable

## Proposing Features

Open an Issue (not PR) with:
- **Motivation:** Why this feature is needed
- **Proposed solution:** How to implement
- **Alternatives considered:** Other approaches
- **Additional context:** Any other relevant information

Discuss with maintainers before implementing large features.

## Code Review Guidelines

All PRs require:
- 2 approvals (at least 1 from core team)
- All CI checks passing
- Code coverage not decreasing
- No merge conflicts

## Community

- **Discussions:** GitHub Discussions for non-urgent questions
- **Chat:** Slack #cognitive-substrate (link in README)
- **Events:** Monthly community call (link in README)

## License

By contributing, you agree to license your contributions under Apache 2.0.
```

### Public Repository Structure
```
github.com/cognitive-substrate/cognitive-substrate/
├── README.md                    # Main entry point
├── CONTRIBUTING.md              # How to contribute
├── CODE_OF_CONDUCT.md          # Community standards
├── SECURITY.md                  # Vulnerability disclosure
├── GOVERNANCE.md                # Decision-making
├── LICENSE                      # Apache 2.0
├── .github/
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.md
│   │   ├── feature_request.md
│   │   └── documentation.md
│   ├── PULL_REQUEST_TEMPLATE.md
│   └── workflows/               # GitHub Actions CI/CD
├── kernel/
├── services/
├── runtime/
├── sdk/
├── docs/
├── tests/
├── benches/
├── BUILD
├── WORKSPACE
└── .gitignore
```

### Issue Template: Bug Report
```markdown
---
name: Bug report
about: Report a bug to help us improve

---

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '...'
3. See error '...'

**Expected behavior**
A clear and concise description of what you expected to happen.

**Environment**
- OS: [e.g., Ubuntu 22.04]
- CS Version: [e.g., 1.0.0]
- Hardware: [e.g., x86_64, ARM64]

**Additional context**
- Logs: (paste relevant logs or core dumps)
- Screenshots: (if applicable)
- Reproducible: (yes/no with example)
```

### PR Template
```markdown
## Description
Brief description of changes.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Related Issue
Fixes #(issue number)

## Testing
Describe testing approach:
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing completed

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings generated
- [ ] Tests pass locally
- [ ] Commit messages clear
```

### SECURITY.md
```markdown
# Security Policy

## Reporting Security Vulnerabilities

Do NOT open a public issue for security vulnerabilities.

Instead, email security@cognitivesubstrate.dev with:
- Vulnerability description
- Affected versions
- Steps to reproduce
- Proof of concept (if available)

We will:
- Acknowledge receipt within 48 hours
- Provide timeline for fix (typically 30 days)
- Credit you in release notes (optional)
- Coordinate disclosure with you

## Supported Versions

| Version | Status | Security Updates |
|---------|--------|------------------|
| 1.x     | Active | Yes             |
| 0.x     | Ended  | No              |
```

## Dependencies
- **Blocked by:** Week 24 Phase 2 completion, Week 32 API Playground done
- **Blocking:** Week 34 benchmarks publication, Week 35-36 launch

## Acceptance Criteria
- [ ] All source files have Apache 2.0 headers
- [ ] CONTRIBUTING.md enables external developers to contribute
- [ ] Code of Conduct provides community standards
- [ ] Issue templates guide bug reporters
- [ ] SECURITY.md process is clear and documented
- [ ] Repository passes GitHub security scanning
- [ ] README provides 5-minute setup experience

## Design Principles Alignment
- **Open Source:** Apache 2.0 enables broad adoption
- **Community:** CONTRIBUTING.md lowers contribution barrier
- **Security:** SECURITY.md enables responsible vulnerability disclosure
- **Governance:** GOVERNANCE.md provides transparent decision-making
