# Engineer 10 — SDK: Tooling, Packaging & Documentation — Week 22

## Phase: 2 (Advanced Debugging Tools & Registry)

## Weekly Objective
Harden cs-pkg registry and CLI. Complete cs-ctl CLI as unified system administration tool. Ensure all 5 debugging tools are fully functional and integrated. Prepare Phase 3 transition.

## Document References
- **Primary:** Section 3.5.3 — cs-pkg: Package Manager, Section 6.3 — Phase 2, Week 20-24
- **Supporting:** Section 3.5.4 (All Debugging Tools)

## Deliverables
- [ ] Registry hardening: rate limiting, abuse prevention, backup/disaster recovery
- [ ] cs-pkg CLI completeness: all advertised features working
- [ ] cs-ctl CLI: unified interface for all system administration tasks
- [ ] Integration tests for all debugging tools with cs-ctl
- [ ] Man pages for cs-pkg, cs-ctl, and all 5 debugging tools
- [ ] Quick-start guides for each debugging tool
- [ ] Performance benchmarks for registry (search, install, publish)
- [ ] Phase 2 retrospective and Phase 3 readiness checklist

## Technical Specifications
### cs-ctl Unified CLI Structure
```bash
cs-ctl                                    # Help
cs-ctl trace <ct_id>                      # Launch cs-trace
cs-ctl replay <core_dump>                 # Launch cs-replay
cs-ctl profile <agent_id>                 # Launch cs-profile
cs-ctl capgraph show <agent_id>           # Launch cs-capgraph
cs-ctl top                                # Launch cs-top
cs-ctl pkg search <query>                 # cs-pkg search
cs-ctl pkg install <package>              # cs-pkg install
cs-ctl pkg publish <path>                 # cs-pkg publish
```

### Registry Hardening
- Rate limiting: 100 requests/minute per IP
- Abuse prevention: spam detection, malware scanning
- Backup: daily backups with 30-day retention
- Disaster recovery: RTO <1 hour, RPO <5 minutes
- CDN: global distribution for faster package delivery

### Integration Tests Coverage
```
cs-trace + cs-ctl: attach to running CT
cs-replay + cs-ctl: load and step through core dump
cs-profile + cs-ctl: profile agent performance
cs-capgraph + cs-ctl: visualize capability graph
cs-top + cs-ctl: monitor system-wide metrics
cs-pkg + cs-ctl: install debugging tool packages
```

### Man Pages Format
```
NAME
    cs-trace - trace CSCI syscalls in real-time

SYNOPSIS
    cs-ctl trace <ct_id> [OPTIONS]

DESCRIPTION
    Attach to running cognitive task and trace all CSCI syscalls...

OPTIONS
    --filter FILTER       Filter syscalls by type/cost
    --output FORMAT       Output format: text, json, binary
    --follow              Continuous stream until CT completes

EXAMPLES
    cs-ctl trace 1001
    cs-ctl trace 1001 --filter "syscall=TOOL_INVOKE"
    cs-ctl trace 1001 --output json > trace.json

SEE ALSO
    cs-ctl(1), cs-top(1), cs-replay(1)
```

## Dependencies
- **Blocked by:** Week 15-20 debugging tools implementation, Week 21 registry launch
- **Blocking:** Phase 3 begins Week 25

## Acceptance Criteria
- [ ] cs-ctl CLI has 0 missing documented commands
- [ ] All 5 debugging tools accessible via cs-ctl
- [ ] Registry rate limiting prevents abuse without blocking legitimate users
- [ ] All man pages render correctly and provide sufficient detail
- [ ] Quick-start guides enable users to debug in <5 minutes
- [ ] Integration tests for all tool combinations pass
- [ ] Phase 2 retrospective completed and lessons learned documented

## Design Principles Alignment
- **Cognitive-Native:** cs-ctl unified interface matches cognitive workflow
- **Debuggability:** All tools accessible from single command-line entry point
- **Isolation by Default:** cs-ctl respects capability boundaries in all tools
- **Developer Experience:** Man pages and quick-start guides enable independent usage
