// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Cognitive journal for recording execution history with redaction capabilities.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Cognitive journal entry recording execution context
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub entry_id: u64,
    pub timestamp: u64,
    pub context: String,
    pub observation: String,
    pub decision: String,
    pub action: String,
    pub outcome: String,
    pub redaction_level: RedactionLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionLevel {
    /// No redaction
    Public,
    /// Internal use only
    Internal,
    /// Redact PII and sensitive data
    Sanitized,
    /// Fully redacted
    HighlyConfidential,
}

impl fmt::Display for RedactionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => write!(f, "Public"),
            Self::Internal => write!(f, "Internal"),
            Self::Sanitized => write!(f, "Sanitized"),
            Self::HighlyConfidential => write!(f, "HighlyConfidential"),
        }
    }
}

impl JournalEntry {
    pub fn new(entry_id: u64, context: String, redaction_level: RedactionLevel) -> Self {
        Self {
            entry_id,
            timestamp: 0,
            context,
            observation: String::new(),
            decision: String::new(),
            action: String::new(),
            outcome: String::new(),
            redaction_level,
        }
    }

    pub fn with_observation(mut self, observation: String) -> Self {
        self.observation = observation;
        self
    }

    pub fn with_decision(mut self, decision: String) -> Self {
        self.decision = decision;
        self
    }

    pub fn with_action(mut self, action: String) -> Self {
        self.action = action;
        self
    }

    pub fn with_outcome(mut self, outcome: String) -> Self {
        self.outcome = outcome;
        self
    }

    /// Get redacted version of entry
    pub fn redacted(&self) -> JournalEntry {
        match self.redaction_level {
            RedactionLevel::Public => self.clone(),
            RedactionLevel::Internal => {
                let mut redacted = self.clone();
                redacted.observation = Self::mask_sensitive(&redacted.observation);
                redacted.outcome = Self::mask_sensitive(&redacted.outcome);
                redacted
            }
            RedactionLevel::Sanitized => {
                let mut redacted = self.clone();
                redacted.observation = Self::mask_sensitive(&redacted.observation);
                redacted.decision = Self::mask_sensitive(&redacted.decision);
                redacted.action = Self::mask_sensitive(&redacted.action);
                redacted.outcome = Self::mask_sensitive(&redacted.outcome);
                redacted
            }
            RedactionLevel::HighlyConfidential => {
                let mut redacted = self.clone();
                redacted.context = String::from("[REDACTED]");
                redacted.observation = String::from("[REDACTED]");
                redacted.decision = String::from("[REDACTED]");
                redacted.action = String::from("[REDACTED]");
                redacted.outcome = String::from("[REDACTED]");
                redacted
            }
        }
    }

    fn mask_sensitive(s: &str) -> String {
        // Simple masking: replace sensitive patterns
        s.replace(|c: char| c.is_digit(10), "*")
    }
}

/// Cognitive journal for recording and managing execution history
#[derive(Debug, Clone)]
pub struct CognitiveJournal {
    entries: Vec<JournalEntry>,
    max_entries: usize,
    current_id: u64,
}

impl CognitiveJournal {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
            current_id: 0,
        }
    }

    /// Record a new journal entry
    pub fn record(&mut self, entry: JournalEntry) -> Result<u64, JournalError> {
        if self.entries.len() >= self.max_entries {
            return Err(JournalError::JournalFull {
                max: self.max_entries,
            });
        }

        let entry_id = self.current_id;
        self.current_id += 1;

        let mut new_entry = entry;
        new_entry.entry_id = entry_id;

        self.entries.push(new_entry);
        Ok(entry_id)
    }

    /// Get entry by ID
    pub fn get(&self, entry_id: u64) -> Option<&JournalEntry> {
        self.entries.iter().find(|e| e.entry_id == entry_id)
    }

    /// Get redacted entry
    pub fn get_redacted(&self, entry_id: u64) -> Option<JournalEntry> {
        self.get(entry_id).map(|e| e.redacted())
    }

    /// List all entries matching redaction level
    pub fn list_by_level(&self, level: RedactionLevel) -> Vec<&JournalEntry> {
        self.entries.iter().filter(|e| e.redaction_level == level).collect()
    }

    /// Get entries in time range
    pub fn get_range(&self, start_time: u64, end_time: u64) -> Vec<&JournalEntry> {
        self.entries
            .iter()
            .filter(|e| e.timestamp >= start_time && e.timestamp <= end_time)
            .collect()
    }

    /// Export redacted entries for external audit
    pub fn export_redacted(&self) -> Vec<JournalEntry> {
        self.entries.iter().map(|e| e.redacted()).collect()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn is_full(&self) -> bool {
        self.entries.len() >= self.max_entries
    }
}

impl Default for CognitiveJournal {
    fn default() -> Self {
        Self::new(10000)
    }
}

/// Redaction engine for systematic data protection
#[derive(Debug, Clone)]
pub struct RedactionEngine {
    patterns: Vec<(String, String)>, // (pattern, replacement)
    redaction_level: RedactionLevel,
}

impl RedactionEngine {
    pub fn new(redaction_level: RedactionLevel) -> Self {
        Self {
            patterns: Vec::new(),
            redaction_level,
        }
    }

    /// Add a redaction pattern
    pub fn add_pattern(&mut self, pattern: String, replacement: String) {
        self.patterns.push((pattern, replacement));
    }

    /// Redact text according to patterns
    pub fn redact(&self, text: &str) -> String {
        match self.redaction_level {
            RedactionLevel::Public => text.to_string(),
            RedactionLevel::HighlyConfidential => String::from("[REDACTED]"),
            _ => {
                let mut result = text.to_string();
                for (pattern, replacement) in &self.patterns {
                    result = result.replace(pattern, replacement);
                }
                result
            }
        }
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

impl Default for RedactionEngine {
    fn default() -> Self {
        Self::new(RedactionLevel::Sanitized)
    }
}

/// Journal errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalError {
    JournalFull { max: usize },
    EntryNotFound,
    InvalidEntry,
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JournalFull { max } => write!(f, "journal full (max: {})", max),
            Self::EntryNotFound => write!(f, "entry not found"),
            Self::InvalidEntry => write!(f, "invalid entry"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_journal_entry() {
        let entry = JournalEntry::new(1, String::from("context"), RedactionLevel::Public)
            .with_observation(String::from("observed X"))
            .with_decision(String::from("decide Y"))
            .with_action(String::from("do Z"))
            .with_outcome(String::from("result W"));

        assert_eq!(entry.entry_id, 1);
        assert_eq!(entry.observation, "observed X");
    }

    #[test]
    fn test_redaction_public() {
        let entry = JournalEntry::new(1, String::from("ctx"), RedactionLevel::Public);
        let redacted = entry.redacted();
        assert_eq!(redacted.context, entry.context);
    }

    #[test]
    fn test_redaction_confidential() {
        let entry = JournalEntry::new(1, String::from("secret"), RedactionLevel::HighlyConfidential)
            .with_observation(String::from("observed"));

        let redacted = entry.redacted();
        assert_eq!(redacted.context, "[REDACTED]");
        assert_eq!(redacted.observation, "[REDACTED]");
    }

    #[test]
    fn test_cognitive_journal() {
        let mut journal = CognitiveJournal::new(100);

        let entry1 = JournalEntry::new(0, String::from("ctx1"), RedactionLevel::Public);
        let entry2 = JournalEntry::new(0, String::from("ctx2"), RedactionLevel::Internal);

        let id1 = journal.record(entry1).unwrap();
        let id2 = journal.record(entry2).unwrap();

        assert_eq!(journal.entry_count(), 2);

        let retrieved = journal.get(id1).unwrap();
        assert_eq!(retrieved.context, "ctx1");
    }

    #[test]
    fn test_journal_full() {
        let mut journal = CognitiveJournal::new(2);

        journal.record(JournalEntry::new(0, String::from("e1"), RedactionLevel::Public)).unwrap();
        journal.record(JournalEntry::new(0, String::from("e2"), RedactionLevel::Public)).unwrap();

        assert!(matches!(
            journal.record(JournalEntry::new(0, String::from("e3"), RedactionLevel::Public)),
            Err(JournalError::JournalFull { .. })
        ));
    }

    #[test]
    fn test_redaction_engine() {
        let mut engine = RedactionEngine::new(RedactionLevel::Sanitized);
        engine.add_pattern(String::from("123"), String::from("***"));

        let result = engine.redact("my number is 123");
        assert_eq!(result, "my number is ***");
    }

    #[test]
    fn test_export_redacted() {
        let mut journal = CognitiveJournal::new(10);

        journal.record(JournalEntry::new(0, String::from("secret"), RedactionLevel::HighlyConfidential)).unwrap();
        journal.record(JournalEntry::new(0, String::from("public"), RedactionLevel::Public)).unwrap();

        let exported = journal.export_redacted();
        assert_eq!(exported.len(), 2);
        assert_eq!(exported[0].context, "[REDACTED]");
    }
}
