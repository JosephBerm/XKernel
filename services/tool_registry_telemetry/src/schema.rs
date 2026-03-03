// Copyright 2026 Cognitive Substrate Project. Apache-2.0 License.
//! Type schema definitions for tool input and output specifications.
//!
//! Defines schemas for validating tool inputs and outputs,
//! ensuring type safety and constraint verification.
//!
//! See Engineering Plan § 2.11.3: Type Schema.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Field definition in a schema.
///
/// Describes a single field including its type and constraints.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDefinition {
    /// Name of the field.
    pub name: String,

    /// Type name of the field (e.g., "string", "int", "array").
    pub type_name: String,

    /// Whether this field is required.
    pub required: bool,

    /// Optional description of the field.
    pub description: Option<String>,
}

impl FieldDefinition {
    /// Creates a new field definition.
    pub fn new(name: impl Into<String>, type_name: impl Into<String>, required: bool) -> Self {
        FieldDefinition {
            name: name.into(),
            type_name: type_name.into(),
            required,
            description: None,
        }
    }

    /// Sets the description for this field.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Validation rule for schema fields.
///
/// Defines constraints on field values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationRule {
    /// Minimum length (for strings) or value (for numbers).
    MinLength(u32),

    /// Maximum length (for strings) or value (for numbers).
    MaxLength(u32),

    /// Regex pattern that value must match.
    Pattern(String),

    /// Value must be one of the allowed values.
    Enum(Vec<String>),

    /// Custom validation rule.
    Custom(String),
}

impl fmt::Display for ValidationRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationRule::MinLength(n) => write!(f, "MinLength({})", n),
            ValidationRule::MaxLength(n) => write!(f, "MaxLength({})", n),
            ValidationRule::Pattern(p) => write!(f, "Pattern({})", p),
            ValidationRule::Enum(_) => write!(f, "Enum"),
            ValidationRule::Custom(c) => write!(f, "Custom({})", c),
        }
    }
}

/// Schema definition for tool inputs or outputs.
///
/// Describes the structure and constraints of data that flows
/// through a tool binding.
///
/// See Engineering Plan § 2.11.3: Type Schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaDefinition {
    /// Name of this schema (e.g., "GitHubIssueInput").
    pub type_name: String,

    /// Fields in this schema.
    pub fields: Vec<FieldDefinition>,

    /// Names of required fields.
    pub required_fields: BTreeSet<String>,

    /// Validation rules for this schema.
    pub validation_rules: Vec<ValidationRule>,
}

impl SchemaDefinition {
    /// Creates a new schema definition.
    pub fn new(type_name: impl Into<String>) -> Self {
        SchemaDefinition {
            type_name: type_name.into(),
            fields: Vec::new(),
            required_fields: BTreeSet::new(),
            validation_rules: Vec::new(),
        }
    }

    /// Adds a field to this schema.
    pub fn add_field(mut self, field: FieldDefinition) -> Self {
        if field.required {
            self.required_fields.insert(field.name.clone());
        }
        self.fields.push(field);
        self
    }

    /// Adds a validation rule to this schema.
    pub fn add_validation_rule(mut self, rule: ValidationRule) -> Self {
        self.validation_rules.push(rule);
        self
    }

    /// Returns the number of fields in this schema.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// Returns the number of required fields.
    pub fn required_field_count(&self) -> usize {
        self.required_fields.len()
    }

    /// Checks if a field name is required.
    pub fn is_field_required(&self, field_name: &str) -> bool {
        self.required_fields.contains(field_name)
    }

    /// Gets a field by name, if it exists.
    pub fn get_field(&self, name: &str) -> Option<&FieldDefinition> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// Complete schema specification for a tool's input and output.
///
/// Specifies both input and output type schemas for a tool binding.
///
/// See Engineering Plan § 2.11.3: Type Schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeSchema {
    /// Schema for tool input.
    pub input_schema: SchemaDefinition,

    /// Schema for tool output.
    pub output_schema: SchemaDefinition,
}

impl TypeSchema {
    /// Creates a new type schema.
    pub fn new(
        input_schema: SchemaDefinition,
        output_schema: SchemaDefinition,
    ) -> Self {
        TypeSchema {
            input_schema,
            output_schema,
        }
    }

    /// Creates a simple schema for a tool with no inputs (void input).
    pub fn no_input(output_type: impl Into<String>) -> Self {
        TypeSchema {
            input_schema: SchemaDefinition::new("void"),
            output_schema: SchemaDefinition::new(output_type),
        }
    }

    /// Creates a simple schema for a tool with no outputs (void output).
    pub fn no_output(input_type: impl Into<String>) -> Self {
        TypeSchema {
            input_schema: SchemaDefinition::new(input_type),
            output_schema: SchemaDefinition::new("void"),
        }
    }

    /// Returns true if this schema has any required input fields.
    pub fn has_required_inputs(&self) -> bool {
        !self.input_schema.required_fields.is_empty()
    }

    /// Returns true if this schema has any required output fields.
    pub fn has_required_outputs(&self) -> bool {
        !self.output_schema.required_fields.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use alloc::string::ToString;
use alloc::vec;

    #[test]
    fn test_field_definition_creation() {
        let field = FieldDefinition::new("name", "string", true);
        assert_eq!(field.name, "name");
        assert_eq!(field.type_name, "string");
        assert!(field.required);
        assert_eq!(field.description, None);
    }

    #[test]
    fn test_field_definition_with_description() {
        let field = FieldDefinition::new("email", "string", true)
            .with_description("User email address");
        assert_eq!(field.description, Some("User email address".to_string()));
    }

    #[test]
    fn test_validation_rule_display() {
        assert_eq!(ValidationRule::MinLength(5).to_string(), "MinLength(5)");
        assert_eq!(ValidationRule::MaxLength(100).to_string(), "MaxLength(100)");
        assert!(ValidationRule::Pattern("^[a-z]+$".to_string())
            .to_string()
            .contains("Pattern"));
        assert_eq!(ValidationRule::Enum(vec![]).to_string(), "Enum");
        assert!(ValidationRule::Custom("custom_check".to_string())
            .to_string()
            .contains("Custom"));
    }

    #[test]
    fn test_schema_definition_creation() {
        let schema = SchemaDefinition::new("UserInput");
        assert_eq!(schema.type_name, "UserInput");
        assert_eq!(schema.field_count(), 0);
        assert_eq!(schema.required_field_count(), 0);
    }

    #[test]
    fn test_schema_definition_add_field() {
        let field1 = FieldDefinition::new("name", "string", true);
        let field2 = FieldDefinition::new("age", "int", false);

        let schema = SchemaDefinition::new("Person")
            .add_field(field1)
            .add_field(field2);

        assert_eq!(schema.field_count(), 2);
        assert_eq!(schema.required_field_count(), 1);
        assert!(schema.is_field_required("name"));
        assert!(!schema.is_field_required("age"));
    }

    #[test]
    fn test_schema_definition_get_field() {
        let field = FieldDefinition::new("email", "string", true);
        let schema = SchemaDefinition::new("User").add_field(field);

        let retrieved = schema.get_field("email");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().type_name, "string");

        let not_found = schema.get_field("phone");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_schema_definition_add_validation_rule() {
        let schema = SchemaDefinition::new("Email")
            .add_validation_rule(ValidationRule::Pattern(".*@.*".to_string()))
            .add_validation_rule(ValidationRule::MaxLength(255));

        assert_eq!(schema.validation_rules.len(), 2);
    }

    #[test]
    fn test_type_schema_creation() {
        let input = SchemaDefinition::new("Input");
        let output = SchemaDefinition::new("Output");
        let schema = TypeSchema::new(input, output);

        assert_eq!(schema.input_schema.type_name, "Input");
        assert_eq!(schema.output_schema.type_name, "Output");
    }

    #[test]
    fn test_type_schema_no_input() {
        let schema = TypeSchema::no_input("Result");
        assert_eq!(schema.input_schema.type_name, "void");
        assert_eq!(schema.output_schema.type_name, "Result");
        assert!(!schema.has_required_inputs());
    }

    #[test]
    fn test_type_schema_no_output() {
        let schema = TypeSchema::no_output("Request");
        assert_eq!(schema.input_schema.type_name, "Request");
        assert_eq!(schema.output_schema.type_name, "void");
        assert!(!schema.has_required_outputs());
    }

    #[test]
    fn test_type_schema_has_required_inputs() {
        let input = SchemaDefinition::new("Input")
            .add_field(FieldDefinition::new("param", "string", true));
        let output = SchemaDefinition::new("Output");
        let schema = TypeSchema::new(input, output);

        assert!(schema.has_required_inputs());
    }

    #[test]
    fn test_type_schema_has_required_outputs() {
        let input = SchemaDefinition::new("Input");
        let output = SchemaDefinition::new("Output")
            .add_field(FieldDefinition::new("result", "string", true));
        let schema = TypeSchema::new(input, output);

        assert!(schema.has_required_outputs());
    }

    #[test]
    fn test_schema_equality() {
        let s1 = SchemaDefinition::new("Test");
        let s2 = SchemaDefinition::new("Test");
        assert_eq!(s1, s2);

        let s3 = SchemaDefinition::new("Different");
        assert_ne!(s1, s3);
    }

    #[test]
    fn test_type_schema_equality() {
        let input = SchemaDefinition::new("Input");
        let output = SchemaDefinition::new("Output");
        let ts1 = TypeSchema::new(input.clone(), output.clone());
        let ts2 = TypeSchema::new(input, output);
        assert_eq!(ts1, ts2);
    }

    #[test]
    fn test_field_definition_equality() {
        let f1 = FieldDefinition::new("name", "string", true);
        let f2 = FieldDefinition::new("name", "string", true);
        assert_eq!(f1, f2);

        let f3 = FieldDefinition::new("name", "int", true);
        assert_ne!(f1, f3);
    }

    #[test]
    fn test_validation_rule_equality() {
        assert_eq!(
            ValidationRule::MinLength(5),
            ValidationRule::MinLength(5)
        );
        assert_ne!(ValidationRule::MinLength(5), ValidationRule::MaxLength(5));
    }

    #[test]
    fn test_complex_schema() {
        let input = SchemaDefinition::new("GitHubIssueInput")
            .add_field(FieldDefinition::new("repo", "string", true))
            .add_field(FieldDefinition::new("title", "string", true))
            .add_field(FieldDefinition::new("body", "string", false))
            .add_validation_rule(ValidationRule::MaxLength(200));

        let output = SchemaDefinition::new("GitHubIssue")
            .add_field(FieldDefinition::new("id", "int", true))
            .add_field(FieldDefinition::new("url", "string", true));

        let schema = TypeSchema::new(input, output);
        assert!(schema.has_required_inputs());
        assert!(schema.has_required_outputs());
    }
}
