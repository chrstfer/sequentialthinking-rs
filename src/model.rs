use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Custom schema generator for nullable fields to emit `anyOf: [T, null]`
/// instead of non-portable `type: [T, "null"]` array forms.
fn nullable_schema<T: JsonSchema>(
    generator: &mut schemars::generate::SchemaGenerator,
) -> schemars::Schema {
    let subschema = generator.subschema_for::<T>();
    let value = serde_json::json!({
        "anyOf": [
            subschema,
            { "type": "null" }
        ]
    });
    serde_json::from_value(value).expect("valid JSON schema for nullable field")
}

/// Custom schema generator for nullable `u32` fields to emit `anyOf: [T, null]`
/// with `minimum: 1` constraint.
fn nullable_u32_min1_schema(
    generator: &mut schemars::generate::SchemaGenerator,
) -> schemars::Schema {
    let subschema = generator.subschema_for::<u32>();
    let mut subschema_val = serde_json::to_value(&subschema).expect("valid u32 schema json");
    if let Some(obj) = subschema_val.as_object_mut() {
        obj.insert("minimum".to_string(), serde_json::json!(1));
    }
    let value = serde_json::json!({
        "anyOf": [
            subschema_val,
            { "type": "null" }
        ]
    });
    serde_json::from_value(value).expect("valid JSON schema for nullable u32 field")
}

/// Input parameters for the `sequentialthinking` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[schemars(deny_unknown_fields)]
pub struct SequentialThinkingInput {
    /// The reasoning, analysis, or deduction for this thought.
    #[serde(alias = "thought")]
    pub thought: String,

    /// True to continue reasoning; false when concluded.
    #[serde(alias = "next_thought_needed")]
    pub next_thought_needed: bool,

    /// 1-based position of this thought in sequence.
    #[serde(alias = "thought_number")]
    #[schemars(range(min = 1))]
    pub thought_number: u32,

    /// Estimated total thoughts needed (adjust dynamically as understanding deepens).
    #[serde(alias = "total_thoughts")]
    #[schemars(range(min = 1))]
    pub total_thoughts: u32,

    /// True if this thought revises an earlier thought.
    #[serde(default, alias = "is_revision")]
    #[schemars(schema_with = "nullable_schema::<bool>")]
    pub is_revision: Option<bool>,

    /// 1-based thought number being revised (required if isRevision is true).
    #[serde(default, alias = "revises_thought")]
    #[schemars(schema_with = "nullable_u32_min1_schema")]
    pub revises_thought: Option<u32>,

    /// 1-based thought number serving as the branch origin.
    #[serde(default, alias = "branch_from_thought")]
    #[schemars(schema_with = "nullable_u32_min1_schema")]
    pub branch_from_thought: Option<u32>,

    /// Branch identifier (e.g. 'alt-hypothesis'). Required when branching.
    #[serde(default, alias = "branch_id")]
    #[schemars(schema_with = "nullable_schema::<String>")]
    pub branch_id: Option<String>,

    /// True if additional thoughts are needed beyond the initial estimate.
    #[serde(default, alias = "needs_more_thoughts")]
    #[schemars(schema_with = "nullable_schema::<bool>")]
    pub needs_more_thoughts: Option<bool>,
}

/// Output response returned from the `sequentialthinking` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SequentialThinkingResponse {
    /// Recorded thought number.
    pub thought_number: u32,

    /// Current total thoughts estimate.
    pub total_thoughts: u32,

    /// Whether further thinking is expected.
    pub next_thought_needed: bool,

    /// Active branch identifiers in this session.
    pub branches: Vec<String>,

    /// Total thoughts recorded in session history.
    pub thought_history_length: usize,

    /// True if totalThoughts was auto-expanded to match thoughtNumber.
    pub total_thoughts_adjusted: bool,

    /// True if an existing thought was updated in-place.
    pub replaced_existing: bool,
}

