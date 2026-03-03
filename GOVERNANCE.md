# Project Governance

## Project Roles

The XKernal project uses a role-based governance model:

### Contributor
- Anyone who submits a pull request to the project
- Can participate in discussions and issue reporting
- Subject to the Contributor Code of Conduct

### Committer
- Approved by the Maintainers or Technical Steering Committee
- Can review and merge pull requests
- Responsible for code quality and project standards
- Can be revoked for violations of community standards

### Maintainer
- Designated authority for releases and project direction
- Handles version management and release coordination
- Manages critical infrastructure and access controls
- Typically 1-3 individuals per project

### Technical Steering Committee (TSC)
- Provides strategic oversight of the project
- Reviews major architectural decisions
- Resolves disputes and escalated issues
- Meets quarterly to discuss project direction
- Makes decisions on breaking changes

## Decision Making Process

### Standard Changes (Lazy Consensus)
Most changes including:
- Bug fixes
- Non-breaking feature additions
- Documentation improvements
- Dependency updates

Use lazy consensus: If no disagreement is raised within 48 hours of PR submission, approval is implicit.

### Breaking Changes (Formal Vote)
Major decisions including:
- API breaking changes
- Version major releases
- Removing supported features
- Significant architectural changes

Require explicit vote by TSC members:
- Minimum 60% approval required
- All TSC members notified with 5-day review period
- Results documented in project records

## Technical Steering Committee

### Meeting Schedule
- Quarterly meetings (every 3 months)
- Ad-hoc meetings for critical issues
- Minutes published within 7 days of meeting

### Responsibilities
- Review quarterly project status
- Approve or reject significant proposals
- Address escalated community concerns
- Plan major releases and milestones

## Conflict Resolution

1. **Discussion**: Initial disagreements resolved through project channels
2. **Escalation**: If unresolved after 5 business days, escalate to Maintainers
3. **Formal Review**: TSC reviews and makes final determination
4. **Transparency**: All decisions documented in project records

## Amendments

Changes to governance require:
- Formal TSC vote
- 70% approval threshold
- 10-day community feedback period
- Public announcement before implementation
