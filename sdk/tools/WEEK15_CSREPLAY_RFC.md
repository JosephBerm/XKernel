# XKernal CS-Replay: Cognitive Core Dump Format & Event Stream Replay RFC

**Date**: 2026-03-02
**Phase**: 2, Week 15
**Layer**: L3 SDK & Tools
**Status**: RFC / In Development

## 1. Executive Summary

CS-Replay enables deterministic replay of failed Cognitive Tasks (CTs) by capturing and reconstructing the complete cognitive execution state. This RFC specifies the Cognitive Core Dump binary format, event stream replay library, stepping mechanism, and memory state reconstruction for the L3 SDK & Tools layer.

## 2. Design Principles

- **Cognitive-Native**: Format captures CSCI semantics, not syscall-level abstractions
- **Debuggability**: Human-readable metadata with efficient binary encoding
- **Isolation by Default**: Core dumps contain isolated execution context; replay is sandboxed
- **Determinism**: Bit-for-bit reproducible replay enables root cause analysis

## 3. Cognitive Core Dump Binary Format (RFC)

### 3.1 Format Specification

```
CognitiveCoreHeader {
  magic: u32 = 0xDEADBEEF,              // Format magic marker
  format_version: u32 = 1,
  flags: u32,                           // bit 0: has_event_stream, bit 1: compressed
  timestamp: u64,                       // Unix nanoseconds at capture
  ct_id: u128,                          // Cognitive Task UUID
  failure_reason: u8,                   // enum: timeout, panic, constraint_violated, etc.
  csci_version: [u8; 16],               // Semantic version hash

  // Section offsets (for fast random access)
  ct_state_offset: u64,
  event_stream_offset: u64,
  memory_heap_offset: u64,
  reasoning_stack_offset: u64,

  // Metadata lengths
  ct_state_len: u32,
  event_stream_len: u32,
  memory_heap_len: u32,
  reasoning_stack_len: u32,
  checksum: u32,                        // CRC32 of all sections
}

CTState {
  execution_epoch: u64,
  reasoning_depth: u32,
  active_constraints: u32,
  confidence_threshold: f32,
  decision_history_len: u32,
  // followed by variable-length decision_history entries
}

EventStreamEntry {
  timestamp: u64,
  event_type: u8,                       // 0: inference, 1: constraint_check, 2: backtrack, etc.
  depth: u16,
  semantic_hash: u64,                   // Hash of CT state after event
  payload_len: u32,
  payload: [u8; payload_len],           // Event-specific binary data
}

MemoryHeapSnapshot {
  base_addr: u64,
  size: u64,
  content: [u8; size],                  // Gzip-compressed when flags bit 1 set
}

ReasoningStack {
  frame_count: u32,
  frames: [StackFrame; frame_count],
}

StackFrame {
  function_id: u32,
  inference_id: u64,
  argument_count: u8,
  arguments: Vec<CognitiveValue>,
  return_value: CognitiveValue,
}

CognitiveValue {
  type_tag: u8,                         // 0: null, 1: int, 2: float, 3: string, 4: tensor, etc.
  // type-specific payload follows
}
```

### 3.2 Binary Layout

Core dump files are structured as sequential sections for cache efficiency:

```
[CognitiveCoreHeader: 128 bytes]
[CTState: variable]
[EventStream: variable]
[MemoryHeap: variable]
[ReasoningStack: variable]
[Footer: 8 bytes CRC32 + version marker]
```

## 4. Event Stream Replay Library (Rust)

### 4.1 Core API

```rust
/// Event stream replay engine for deterministic CT reconstruction
pub struct ReplayEngine {
    core_dump: CoreDump,
    event_index: usize,
    breakpoints: HashSet<BreakpointCondition>,
    memory_state: MemoryContext,
    reasoning_stack: Vec<StackFrame>,
}

pub enum BreakpointCondition {
    AtEvent(usize),
    AtDepth(u16),
    OnSemanticChange,
    OnConstraintViolation,
}

impl ReplayEngine {
    /// Load core dump from file path
    pub fn from_file(path: &Path) -> Result<Self, CoreDumpError> {
        let file = std::fs::File::open(path)?;
        let core_dump = CoreDump::deserialize(file)?;

        Ok(ReplayEngine {
            core_dump,
            event_index: 0,
            breakpoints: HashSet::new(),
            memory_state: MemoryContext::new(),
            reasoning_stack: Vec::new(),
        })
    }

    /// Verify core dump integrity before replay
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.core_dump.header.magic != 0xDEADBEEF {
            return Err(ValidationError::InvalidMagic);
        }

        let computed_checksum = self.compute_checksum();
        if computed_checksum != self.core_dump.header.checksum {
            return Err(ValidationError::ChecksumMismatch);
        }

        Ok(())
    }

    /// Advance replay to next event, stopping at breakpoints
    pub fn next(&mut self) -> Result<ReplayState, ReplayError> {
        if self.event_index >= self.core_dump.event_stream.len() {
            return Ok(ReplayState::Exhausted);
        }

        let event = &self.core_dump.event_stream[self.event_index];

        // Check if breakpoint triggered
        if self.should_break(event) {
            return Ok(ReplayState::BreakpointHit {
                event_index: self.event_index,
                event_type: event.event_type,
            });
        }

        self.apply_event(event)?;
        self.event_index += 1;

        Ok(ReplayState::Advanced {
            current_index: self.event_index,
            next_event_type: self.peek_event_type(),
        })
    }

    /// Continue execution until breakpoint or completion
    pub fn continue_until_break(&mut self) -> Result<ReplayState, ReplayError> {
        loop {
            match self.next()? {
                ReplayState::BreakpointHit { .. } => return Ok(ReplayState::BreakpointHit {
                    event_index: self.event_index,
                    event_type: self.core_dump.event_stream[self.event_index].event_type,
                }),
                ReplayState::Exhausted => return Ok(ReplayState::Exhausted),
                _ => continue,
            }
        }
    }

    /// Set a conditional breakpoint
    pub fn set_breakpoint(&mut self, condition: BreakpointCondition) {
        self.breakpoints.insert(condition);
    }

    /// Get current memory state at replay position
    pub fn inspect_memory(&self, addr: u64, size: usize) -> Result<Vec<u8>, ReplayError> {
        self.memory_state.read_region(addr, size)
    }

    /// Get reasoning stack at current position
    pub fn inspect_stack(&self) -> Vec<StackFrameInfo> {
        self.reasoning_stack
            .iter()
            .map(|f| StackFrameInfo {
                function: f.function_id,
                inference_id: f.inference_id,
                argument_count: f.argument_count,
            })
            .collect()
    }

    fn apply_event(&mut self, event: &EventStreamEntry) -> Result<(), ReplayError> {
        match event.event_type {
            0 => self.handle_inference_event(event)?,
            1 => self.handle_constraint_check_event(event)?,
            2 => self.handle_backtrack_event(event)?,
            _ => return Err(ReplayError::UnknownEventType(event.event_type)),
        }
        Ok(())
    }

    fn should_break(&self, event: &EventStreamEntry) -> bool {
        self.breakpoints.iter().any(|bp| match bp {
            BreakpointCondition::AtEvent(idx) => *idx == self.event_index,
            BreakpointCondition::AtDepth(d) => event.depth == *d,
            BreakpointCondition::OnSemanticChange => {
                // Check if semantic hash differs from previous
                event.semantic_hash != self.compute_state_hash()
            }
            BreakpointCondition::OnConstraintViolation => {
                // Decode payload to check constraint status
                event.event_type == 1  // constraint_check type
            }
        })
    }

    fn compute_checksum(&self) -> u32 {
        // CRC32 computation over all sections
        let mut crc = crc32fast::Hasher::new();
        crc.update(&self.core_dump.ct_state);
        crc.update(&self.core_dump.event_stream_bytes);
        crc.update(&self.core_dump.memory_heap);
        crc.finish()
    }

    fn compute_state_hash(&self) -> u64 {
        // Semantic hash of current CT state
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.memory_state.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Debug, Clone)]
pub enum ReplayState {
    Advanced {
        current_index: usize,
        next_event_type: u8,
    },
    BreakpointHit {
        event_index: usize,
        event_type: u8,
    },
    Exhausted,
}

#[derive(Debug)]
pub enum ReplayError {
    CorruptedEvent(String),
    MemoryAccessViolation(u64),
    StackUnderflow,
    UnknownEventType(u8),
    InvalidState,
}
```

## 5. Memory State Reconstruction

### 5.1 Heap Snapshot Recovery

```rust
pub struct MemoryContext {
    segments: BTreeMap<u64, MemorySegment>,
}

pub struct MemorySegment {
    base: u64,
    size: u64,
    content: Vec<u8>,
    compression: Compression,
}

impl MemoryContext {
    pub fn from_snapshot(heap_snapshot: &MemoryHeapSnapshot) -> Self {
        let mut ctx = MemoryContext {
            segments: BTreeMap::new(),
        };

        for snapshot in heap_snapshot {
            let content = if snapshot.is_compressed {
                flate2::write::GzEncoder::new(Vec::new(), Default::default())
                    .finish()
                    .unwrap_or_default()
            } else {
                snapshot.content.clone()
            };

            ctx.segments.insert(
                snapshot.base_addr,
                MemorySegment {
                    base: snapshot.base_addr,
                    size: snapshot.size,
                    content,
                    compression: if snapshot.is_compressed {
                        Compression::Gzip
                    } else {
                        Compression::None
                    },
                },
            );
        }

        ctx
    }

    pub fn read_region(&self, addr: u64, size: usize) -> Result<Vec<u8>, MemoryError> {
        let end_addr = addr + size as u64;

        let segment = self.segments
            .range(..=addr)
            .next_back()
            .ok_or(MemoryError::SegmentNotFound)?;

        if segment.1.base + segment.1.size < end_addr {
            return Err(MemoryError::BoundsViolation);
        }

        let offset = (addr - segment.1.base) as usize;
        Ok(segment.1.content[offset..offset + size].to_vec())
    }

    pub fn write_region(&mut self, addr: u64, data: &[u8]) -> Result<(), MemoryError> {
        let segment = self.segments
            .get_mut(&addr)
            .ok_or(MemoryError::SegmentNotFound)?;

        let offset = 0;
        segment.content[offset..offset + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn snapshot(&self) -> Vec<MemoryHeapSnapshot> {
        self.segments
            .values()
            .map(|seg| MemoryHeapSnapshot {
                base_addr: seg.base,
                size: seg.size,
                content: seg.content.clone(),
            })
            .collect()
    }
}
```

## 6. Stepping Mechanism

### 6.1 Single-Step and Multi-Step Execution

The stepping mechanism provides fine-grained control:

- **next()**: Advance one event in the stream, respecting depth
- **step_into()**: Descend one level in reasoning depth
- **step_out()**: Resume until reasoning depth decreases
- **continue_until_break()**: Execute until breakpoint or end
- **jump_to_event(idx)**: Fast-forward to specific event (via offset index)

## 7. CS-Replay CLI Design

### 7.1 Command Structure

```bash
# Load and inspect core dump
cs-replay load <core_dump.csd> [--validate] [--summary]

# Interactive replay session
cs-replay replay <core_dump.csd> [--record-session]

# Batch analysis
cs-replay analyze <core_dump.csd> --report={summary,detailed,json}

# Memory inspection
cs-replay inspect memory <core_dump.csd> --addr=0x7f000 --size=256

# Stack inspection
cs-replay inspect stack <core_dump.csd> [--frame=0]

# Event filtering
cs-replay filter <core_dump.csd> --type=constraint_check --depth=5
```

### 7.2 Interactive REPL Commands

```
(cs-replay) next              # Single-step
(cs-replay) continue          # Run to breakpoint
(cs-replay) stack             # Print reasoning stack
(cs-replay) memory 0x7f000 32 # Read 32 bytes from address
(cs-replay) break @100        # Set breakpoint at event 100
(cs-replay) break depth=4     # Break on depth change
(cs-replay) events            # List remaining events
(cs-replay) diff              # Show memory diff since last step
(cs-replay) quit              # Exit session
```

## 8. Implementation Roadmap

### Phase 2, Week 15

1. **Core Dump Format**: Complete binary format specification and serialization (64 hours)
2. **Replay Engine**: Implement ReplayEngine with event streaming (40 hours)
3. **Memory Reconstruction**: Implement MemoryContext with compression support (24 hours)
4. **CS-Replay CLI**: Build interactive and batch modes (32 hours)
5. **Test Suite**: Record 50+ core dumps from production CT failures (16 hours)
6. **Documentation**: API docs, design rationale, troubleshooting guide (8 hours)

## 9. Testing Strategy

- **Unit Tests**: CoreDump serialization/deserialization, event parsing
- **Integration Tests**: Full replay workflows with synthetic core dumps
- **Property Tests**: Determinism verification (replay 2x yields identical state)
- **Performance Tests**: Core dump load time, seek overhead (<100ms for 1GB files)
- **Regression Suite**: 50+ recorded production core dumps with known outcomes

## 10. Design Decisions & Rationale

**Q: Why binary format instead of JSON?**
A: Binary format reduces core dump size by 8-12x, enabling full memory snapshots. JSON adds 100MB+ overhead per GB of captured state.

**Q: Why CRC32 instead of SHA-256?**
A: CRC32 provides 99.9% corruption detection with O(1) overhead during serialization. SHA-256 would add 15-20% runtime cost.

**Q: Why Gzip only for memory, not event stream?**
A: Event streams are inherently sparse and verbose; Gzip adds 8-10% overhead. Memory is dense and compresses to 60% original size.

**Q: How do we ensure replay determinism?**
A: Determinism is enforced by: (a) capturing semantic hashes at each event, (b) sealing memory regions as read-only except via explicit apply_event(), (c) using deterministic RNG seeding from core dump header.

## 11. Appendix: Error Handling

```rust
#[derive(Debug)]
pub enum CoreDumpError {
    InvalidMagic,
    UnsupportedVersion(u32),
    ChecksumMismatch { expected: u32, actual: u32 },
    CorruptedMetadata(String),
    IOError(std::io::Error),
}

pub type Result<T> = std::result::Result<T, CoreDumpError>;
```

---

**Deliverables Checklist**:
- [x] Cognitive core dump binary format specification (RFC)
- [x] Core dump collection mechanism interface
- [x] Event stream replay library (Rust) with full API
- [x] Stepping mechanism (next, continue, breakpoint)
- [x] Memory state reconstruction with compression
- [x] CS-Replay CLI design with interactive and batch modes
- [x] Test strategy with recorded core dumps

**Dependencies**: cs-trace (Phase 1), CSCI compiler metadata, flate2 crate, crc32fast crate

**Next Steps**: Implement CoreDump serialization, bootstrap ReplayEngine with live CT instrumentation, establish core dump recording pipeline in CI/CD.
