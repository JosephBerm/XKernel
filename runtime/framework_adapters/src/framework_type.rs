// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.

//! # Framework Type Enumeration
//!
//! Identifies and categorizes external framework types supported by the adapter subsystem.
//! Sec 4.2: Framework Type Classification


/// Enumeration of supported external frameworks.
/// Sec 4.2: Framework Support Matrix
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrameworkType {
    /// LangChain: Python-based agentic framework
    /// Sec 4.3: LangChain Concept Mapping
    LangChain,

    /// Semantic Kernel: Microsoft's C# semantic orchestration framework
    /// Sec 4.3: Semantic Kernel Concept Mapping
    SemanticKernel,

    /// CrewAI: Role-based multi-agent orchestration framework
    /// Sec 4.3: CrewAI Concept Mapping
    CrewAI,

    /// AutoGen: Microsoft's conversational AI framework
    /// Sec 4.3: AutoGen Concept Mapping
    AutoGen,
}

impl FrameworkType {
    /// Returns the string representation of the framework type.
    /// Sec 4.2: Framework Identification
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameworkType::LangChain => "langchain",
            FrameworkType::SemanticKernel => "semantic_kernel",
            FrameworkType::CrewAI => "crewai",
            FrameworkType::AutoGen => "autogen",
        }
    }

    /// Attempts to parse a framework type from a string.
    /// Sec 4.2: Framework Type Parsing
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "langchain" | "lc" => Some(FrameworkType::LangChain),
            "semantic_kernel" | "sk" | "semantickernel" => Some(FrameworkType::SemanticKernel),
            "crewai" => Some(FrameworkType::CrewAI),
            "autogen" => Some(FrameworkType::AutoGen),
            _ => None,
        }
    }

    /// Returns a description of the framework.
    /// Sec 4.2: Framework Metadata
    pub fn description(&self) -> &'static str {
        match self {
            FrameworkType::LangChain => {
                "LangChain: Python-based agentic framework for LLM orchestration"
            }
            FrameworkType::SemanticKernel => {
                "Semantic Kernel: C# framework for semantic function orchestration"
            }
            FrameworkType::CrewAI => {
                "CrewAI: Role-based multi-agent framework for collaborative automation"
            }
            FrameworkType::AutoGen => {
                "AutoGen: Conversational AI framework for multi-agent interactions"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_type_as_str() {
        assert_eq!(FrameworkType::LangChain.as_str(), "langchain");
        assert_eq!(FrameworkType::SemanticKernel.as_str(), "semantic_kernel");
        assert_eq!(FrameworkType::CrewAI.as_str(), "crewai");
        assert_eq!(FrameworkType::AutoGen.as_str(), "autogen");
    }

    #[test]
    fn test_framework_type_from_string() {
        assert_eq!(FrameworkType::from_string("langchain"), Some(FrameworkType::LangChain));
        assert_eq!(FrameworkType::from_string("LANGCHAIN"), Some(FrameworkType::LangChain));
        assert_eq!(FrameworkType::from_string("lc"), Some(FrameworkType::LangChain));
        assert_eq!(FrameworkType::from_string("semantic_kernel"), Some(FrameworkType::SemanticKernel));
        assert_eq!(FrameworkType::from_string("sk"), Some(FrameworkType::SemanticKernel));
        assert_eq!(FrameworkType::from_string("crewai"), Some(FrameworkType::CrewAI));
        assert_eq!(FrameworkType::from_string("autogen"), Some(FrameworkType::AutoGen));
        assert_eq!(FrameworkType::from_string("unknown"), None);
    }

    #[test]
    fn test_framework_type_description() {
        let desc = FrameworkType::LangChain.description();
        assert!(desc.contains("LangChain"));
        assert!(desc.contains("Python"));
    }
}
