# Contributing to XKernal

Thank you for your interest in contributing to XKernal! This document provides guidelines for participating in our open-source project.

## Prerequisites

Before getting started, ensure you have installed:

- **Rust** 1.75 or later
- **Node.js** 20 or later
- **.NET** 8.0 or later

## Development Setup

### Clone the Repository

```bash
git clone https://github.com/xkernal/xkernal.git
cd xkernal
```

### Build the Project

```bash
cargo build
npm install
dotnet build
```

## Branch Naming Convention

Use the following prefixes when creating branches:

- `feature/` - New features
- `bugfix/` - Bug fixes
- `docs/` - Documentation updates

Example: `feature/user-authentication` or `bugfix/memory-leak`

## Pull Request Workflow

1. **Fork** the repository on GitHub
2. **Create a branch** using the naming conventions above
3. **Write code** following our coding standards
4. **Test thoroughly** to ensure all tests pass
5. **Submit a Pull Request** with a clear description of changes

## Commit Messages

Follow the Conventional Commits specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Example: `feat(core): add async request handling`

## Coding Standards

### Rust

- Run `cargo fmt` before committing
- Enforce linting with `cargo clippy -D warnings`

### JavaScript/TypeScript

- Format code with `prettier`
- Lint with `eslint`

### .NET

- Follow Microsoft C# coding conventions
- Use `dotnet format` for code formatting

## Testing Requirements

- Maintain **80%+ code coverage**
- All tests must pass before submission
- Run tests locally: `cargo test`, `npm test`, `dotnet test`

## Code Review Process

- All PRs require **2 approvals** from maintainers
- Expected response time: **24-48 hours**
- Address feedback promptly and push updates to the same branch

## Questions?

Open an issue or contact the maintainers. We're here to help!

Happy contributing!
