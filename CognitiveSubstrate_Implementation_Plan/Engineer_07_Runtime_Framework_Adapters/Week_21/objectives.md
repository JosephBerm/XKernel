# Engineer 7 — Runtime: Framework Adapters — Week 21
## Phase: Phase 2 (Multi-Framework: AutoGen Adapter)
## Weekly Objective
Implement AutoGen adapter. Translate conversations to SemanticChannels. Map functions to CTs. Support multi-agent conversations and human-in-the-loop scenarios. Validate AutoGen agents on kernel.

## Document References
- **Primary:** Section 6.3 — Phase 2, Week 15-18 (Complete LangChain + SK adapters, CrewAI adapter)
- **Supporting:** Section 3.4.1 — Framework Adapters

## Deliverables
- [ ] AutoGen adapter implementation (70%): conversation, function, and agent structure translation
- [ ] GroupChat-to-SemanticChannel translation: multi-turn conversations mapped to typed IPC
- [ ] ConversableAgent mapping: agent role and capabilities → CSCI agent context
- [ ] Function-to-CT translation: function definitions → CTs with argument mapping and return value handling
- [ ] Conversation history management: maintain conversation state, thread safety
- [ ] Human-in-the-loop support: user proxy agents → input channels for human feedback
- [ ] Multi-turn dialogue handling: maintain state across conversation turns, context preservation
- [ ] Validation tests (10+): simple conversations, function calls, multi-agent dialogue, error scenarios
- [ ] AutoGen MVP scenario: multi-agent conversation with functions on Cognitive Substrate

## Technical Specifications
- GroupChat translation: each group chat message → SemanticChannel message, participants → channel subscribers
- ConversableAgent mapping: agent name/role → CT agent_id, registered_functions → tool bindings
- Function translation: AutoGen function → CT with typed input/output, implements function execution
- Conversation state: maintain message history, track participants, handle async message delivery
- Human-in-the-loop: UserProxyAgent messages → input channel, human response → continue conversation
- Multi-turn handling: each message round → CT spawns to handle (if function call) or respond
- Message format: preserve AutoGen message structure, translate to SemanticChannel IPC
- Error handling: invalid functions, missing participants, timeout on human input
- MVP scenario: coding assistant with 2 agents (assistant, executor) and human oversight

## Dependencies
- **Blocked by:** Week 20
- **Blocking:** Week 22, Week 23, Week 24

## Acceptance Criteria
- AutoGen adapter 70% complete with core conversation features functional
- GroupChat → SemanticChannel translation working
- Functions correctly mapped to CTs
- 10+ validation tests passing
- Human-in-the-loop scenario working (timeout handling)
- Multi-turn conversations maintaining state
- AutoGen MVP scenario successfully executing on kernel
- Architecture supports extensible agent types

## Design Principles Alignment
- **Conversation Native:** Translate AutoGen conversations directly to SemanticChannels
- **Function Agnostic:** Support flexible function definitions and execution
- **Human Centric:** Human-in-the-loop naturally translates to input channels
