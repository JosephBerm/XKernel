# Engineer 3 — Kernel: IPC, Signals, Exceptions & Checkpointing — Week 21

## Phase: PHASE 2 — Optimization & Integration

## Weekly Objective

Integrate with SDK layer: ensure all IPC, signals, exceptions, and checkpointing subsystems work seamlessly through CognitiveSubstrateSDK. Verify end-to-end flows with real application code.

## Document References
- **Primary:** Section 3.2.4-3.2.8 (All IPC, Signals, Exceptions, Checkpointing)
- **Supporting:** Section 2.6-2.12 (Subsystem Designs), Section 6.2 (Exit Criteria)

## Deliverables
- [ ] SDK wrapper layer: type-safe abstractions for all syscalls
- [ ] IPC convenience APIs: simplified channel creation and messaging
- [ ] Signal handler registration: SDK API for registering signal handlers
- [ ] Exception handler registration: SDK API for exception handlers
- [ ] Checkpoint management: SDK API for creating and restoring checkpoints
- [ ] Protocol negotiation: SDK handles protocol selection automatically
- [ ] Error handling: consistent error types across all SDK APIs
- [ ] Documentation: SDK user guide with examples
- [ ] Integration tests: end-to-end application scenarios
- [ ] Performance validation: verify SDK overhead is < 5%

## Technical Specifications

### SDK Type-Safe Wrapper Layer
```
pub struct CognitiveSubstrateSDK {
    kernel_handle: KernelHandle,
    current_ct: ContextThreadRef,
    channel_cache: HashMap<ChannelId, CachedChannelInfo>,
}

impl CognitiveSubstrateSDK {
    pub fn new() -> Result<Self, SDKError> {
        Ok(Self {
            kernel_handle: KernelHandle::new()?,
            current_ct: get_current_context_thread()?,
            channel_cache: HashMap::new(),
        })
    }

    // Safe syscall wrappers with error handling
    pub fn chan_open(
        &mut self,
        protocol_hint: Option<Protocol>,
        remote_endpoint: ContextThreadRef,
    ) -> Result<Channel, SDKError> {
        let channel_id = unsafe {
            syscall::chan_open(protocol_hint, remote_endpoint)?
        };

        let channel = Channel {
            id: channel_id,
            endpoint: remote_endpoint,
            sdk: self,
        };

        self.channel_cache.insert(channel_id, CachedChannelInfo {
            endpoint: remote_endpoint,
            protocol: protocol_hint,
        });

        Ok(channel)
    }

    pub fn signal_handler_registry(&self) -> SignalHandlerRegistry {
        SignalHandlerRegistry::new(self.kernel_handle.clone())
    }

    pub fn exception_handler_registry(&self) -> ExceptionHandlerRegistry {
        ExceptionHandlerRegistry::new(self.kernel_handle.clone())
    }

    pub fn checkpoint_manager(&self) -> CheckpointManager {
        CheckpointManager::new(self.kernel_handle.clone(), self.current_ct)
    }
}
```

### IPC Convenience APIs
```
pub struct Channel {
    pub id: ChannelId,
    pub endpoint: ContextThreadRef,
    pub sdk: *const CognitiveSubstrateSDK,
}

impl Channel {
    pub fn send<T: Serialize>(&self, message: &T) -> Result<(), SendError> {
        let serialized = serde_json::to_vec(message)?;
        unsafe {
            syscall::chan_send(self.id, &serialized)?
        };
        Ok(())
    }

    pub fn recv<T: DeserializeOwned>(&self, timeout_ms: u64) -> Result<T, RecvError> {
        let response = unsafe {
            syscall::chan_recv(self.id, timeout_ms)?
        };
        let deserialized = serde_json::from_slice(&response)?;
        Ok(deserialized)
    }

    pub fn request_response<Req, Resp>(
        &self,
        request: &Req,
    ) -> Result<Resp, RequestResponseError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        self.send(request)?;
        let response = self.recv(REQUEST_RESPONSE_TIMEOUT_MS)?;
        Ok(response)
    }
}

pub struct PubSubTopic {
    pub topic_id: TopicId,
    pub sdk: *const CognitiveSubstrateSDK,
}

impl PubSubTopic {
    pub fn publish<T: Serialize>(&self, message: &T) -> Result<usize, PublishError> {
        let serialized = serde_json::to_vec(message)?;
        let subscriber_count = unsafe {
            syscall::pub_publish(self.topic_id, &serialized)?
        };
        Ok(subscriber_count)
    }

    pub fn subscribe(&self) -> Result<PubSubSubscription, SubscribeError> {
        let subscription_id = unsafe {
            syscall::pub_subscribe(self.topic_id)?
        };
        Ok(PubSubSubscription {
            topic_id: self.topic_id,
            subscription_id,
            sdk: self.sdk,
        })
    }
}

impl PubSubSubscription {
    pub fn recv<T: DeserializeOwned>(&self, timeout_ms: u64) -> Result<T, RecvError> {
        let message = unsafe {
            syscall::pub_receive(self.topic_id, timeout_ms)?
        };
        let deserialized = serde_json::from_slice(&message)?;
        Ok(deserialized)
    }
}
```

### Signal Handler Registration SDK
```
pub struct SignalHandlerRegistry {
    kernel_handle: KernelHandle,
    handlers: HashMap<CognitiveSignal, Box<dyn SignalHandler>>,
}

pub trait SignalHandler: Send + Sync {
    fn handle(&self, signal: &CognitiveSignal) -> SignalHandlerResult;
}

impl SignalHandlerRegistry {
    pub fn register<H: SignalHandler + 'static>(
        &mut self,
        signal: CognitiveSignal,
        handler: H,
    ) -> Result<(), RegisterError> {
        // Validate handler for signal type
        match signal {
            CognitiveSignal::SigTerminate => {
                return Err(RegisterError::CannotRegisterTerminate);
            }
            _ => {}
        }

        // Create handler wrapper (safe Rust -> unsafe syscall)
        let handler_ptr = Box::into_raw(Box::new(handler));
        unsafe {
            syscall::sig_register(signal, handler_ptr as *const ())?;
        }

        self.handlers.insert(signal, unsafe {
            Box::from_raw(handler_ptr)
        });

        Ok(())
    }
}

// Example: Custom signal handler
struct MySignalHandler;

impl SignalHandler for MySignalHandler {
    fn handle(&self, signal: &CognitiveSignal) -> SignalHandlerResult {
        match signal {
            CognitiveSignal::SigDeadlineWarn => {
                eprintln!("Deadline warning received!");
                SignalHandlerResult::Continue
            }
            CognitiveSignal::SigCheckpoint => {
                eprintln!("Checkpointing...");
                SignalHandlerResult::Continue
            }
            _ => SignalHandlerResult::Continue,
        }
    }
}

// Usage
let mut registry = sdk.signal_handler_registry();
registry.register(CognitiveSignal::SigDeadlineWarn, MySignalHandler)?;
```

### Exception Handler Registration SDK
```
pub struct ExceptionHandlerRegistry {
    kernel_handle: KernelHandle,
}

pub trait ExceptionHandler: Send + Sync {
    fn handle(&self, context: &ExceptionContext) -> ExceptionHandlerResult;
}

impl ExceptionHandlerRegistry {
    pub fn register<H: ExceptionHandler + 'static>(
        &mut self,
        handler: H,
    ) -> Result<(), RegisterError> {
        let handler_ptr = Box::into_raw(Box::new(handler));
        unsafe {
            syscall::exc_register(handler_ptr as *const ())?;
        }
        Ok(())
    }
}

// Example: Custom exception handler
struct MyExceptionHandler;

impl ExceptionHandler for MyExceptionHandler {
    fn handle(&self, context: &ExceptionContext) -> ExceptionHandlerResult {
        match context.exception {
            CognitiveException::ToolCallFailed(_) => {
                eprintln!("Tool call failed; retrying...");
                ExceptionHandlerResult::Retry(RetryPolicy::default())
            }
            CognitiveException::ContextOverflow(_) => {
                eprintln!("Context overflow; evicting old data...");
                ExceptionHandlerResult::Continue
            }
            _ => ExceptionHandlerResult::Escalate(get_supervisor_ref()),
        }
    }
}

// Usage
let mut registry = sdk.exception_handler_registry();
registry.register(MyExceptionHandler)?;
```

### Checkpoint Management SDK
```
pub struct CheckpointManager {
    kernel_handle: KernelHandle,
    current_ct: ContextThreadRef,
    checkpoint_ids: VecDeque<CheckpointId>,
}

impl CheckpointManager {
    pub fn create_checkpoint(&mut self) -> Result<CheckpointId, CheckpointError> {
        let cp_id = unsafe {
            syscall::ct_checkpoint()?
        };
        self.checkpoint_ids.push_back(cp_id);
        Ok(cp_id)
    }

    pub fn restore_from_checkpoint(&self, cp_id: CheckpointId) -> Result<(), RestoreError> {
        unsafe {
            syscall::ct_resume(cp_id)?
        };
        Ok(())
    }

    pub fn restore_from_latest(&self) -> Result<(), RestoreError> {
        let cp_id = self.checkpoint_ids.back()
            .ok_or(RestoreError::NoCheckpointAvailable)?;
        self.restore_from_checkpoint(*cp_id)
    }

    pub fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>, ListError> {
        // Query kernel for checkpoint metadata
        let checkpoints = unsafe {
            syscall::ct_list_checkpoints(self.current_ct.ct_id)?
        };
        Ok(checkpoints)
    }
}

pub struct CheckpointInfo {
    pub id: CheckpointId,
    pub timestamp: Timestamp,
    pub memory_size: usize,
    pub is_available: bool,
}
```

### End-to-End Application Example
```
pub async fn example_reasoning_agent(sdk: &mut CognitiveSubstrateSDK) -> Result<(), SDKError> {
    // 1. Create channel to supervisor
    let supervisor_channel = sdk.chan_open(Some(Protocol::ReAct), SUPERVISOR_REF)?;

    // 2. Register signal handlers
    let mut signal_registry = sdk.signal_handler_registry();
    signal_registry.register(CognitiveSignal::SigCheckpoint, CheckpointHandler)?;

    // 3. Register exception handlers
    let mut exc_registry = sdk.exception_handler_registry();
    exc_registry.register(MyExceptionHandler)?;

    // 4. Get checkpoint manager
    let mut checkpoint_mgr = sdk.checkpoint_manager();

    // 5. Main reasoning loop
    loop {
        // Observe
        let observation = get_observation()?;

        // Checkpoint before reasoning
        checkpoint_mgr.create_checkpoint()?;

        // Think and act
        let thought = reason_about(&observation)?;
        let action = choose_action(&thought)?;

        // Send to supervisor for feedback
        let request = ReActMessage {
            thought: thought.clone(),
            action: action.clone(),
            observation: observation.clone(),
        };

        match supervisor_channel.send(&request) {
            Ok(()) => {
                let response: ReActMessage = supervisor_channel.recv(5000)?;
                process_feedback(&response)?;
            }
            Err(e) => {
                eprintln!("IPC failed: {:?}", e);
                // Exception handler will be invoked automatically
                // Handler can decide to retry, rollback, etc.
            }
        }

        // Check if deadline approaching
        if time_remaining_ms() < 1000 {
            break;
        }
    }

    Ok(())
}
```

## Dependencies
- **Blocked by:** Week 1-20 (All implementation work)
- **Blocking:** Week 22-24 (Benchmarking & Launch)

## Acceptance Criteria
1. All CSCI syscalls have type-safe SDK wrappers
2. IPC APIs automatically handle serialization/deserialization
3. Signal and exception handlers registered via trait objects
4. Checkpoint management provides convenient API
5. Example applications work with SDK layer
6. SDK overhead < 5% vs direct syscalls
7. Compile errors prevent misuse of APIs
8. Documentation includes multiple examples
9. Integration tests pass for end-to-end scenarios
10. All functionality from Weeks 1-20 accessible via SDK

## Design Principles Alignment
- **Type Safety:** Rust trait system prevents API misuse
- **Convenience:** Automatic serialization/deserialization hides complexity
- **Ergonomics:** Simple, intuitive API for application developers
- **Transparency:** SDK layer is thin; performance close to direct syscalls
