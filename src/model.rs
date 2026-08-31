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
    /// Your current thinking step. This contains the substantive analysis,
    /// reasoning, hypothesis generation, verification, or reflection for this step.
    #[serde(alias = "thought")]
    pub thought: String,

    /// Whether another thought step is required after this one.
    /// Set to `true` to continue reasoning, or `false` only when the problem is fully
    /// resolved and a final satisfactory conclusion is reached.
    #[serde(alias = "next_thought_needed")]
    pub next_thought_needed: bool,

    /// Current thought number in sequence (1-based integer, e.g. 1, 2, 3).
    /// Can extend beyond the initial total estimate if additional thinking is needed.
    #[serde(alias = "thought_number")]
    #[schemars(range(min = 1))]
    pub thought_number: u32,

    /// Current estimated total number of thoughts required (integer >= 1).
    /// Can be dynamically adjusted up or down as understanding deepens.
    #[serde(alias = "total_thoughts")]
    #[schemars(range(min = 1))]
    pub total_thoughts: u32,

    /// Whether this thought revises, questions, or corrects previous thinking steps.
    /// Optional (defaults to false).
    #[serde(default, alias = "is_revision")]
    #[schemars(schema_with = "nullable_schema::<bool>")]
    pub is_revision: Option<bool>,

    /// When `isRevision` is true, the specific 1-based thought number being reconsidered or amended.
    /// Optional.
    #[serde(default, alias = "revises_thought")]
    #[schemars(schema_with = "nullable_u32_min1_schema")]
    pub revises_thought: Option<u32>,

    /// When exploring an alternative hypothesis or branch, the 1-based thought number
    /// that serves as the branching origin point. Optional.
    #[serde(default, alias = "branch_from_thought")]
    #[schemars(schema_with = "nullable_u32_min1_schema")]
    pub branch_from_thought: Option<u32>,

    /// A descriptive identifier or label for the branch being created or continued
    /// (e.g. 'approach-b', 'alt-hypothesis'). Optional.
    #[serde(default, alias = "branch_id")]
    #[schemars(schema_with = "nullable_schema::<String>")]
    pub branch_id: Option<String>,

    /// Set to `true` if you reached what seemed like the end of the planned sequence
    /// but realize further thought steps are needed. Optional.
    #[serde(default, alias = "needs_more_thoughts")]
    #[schemars(schema_with = "nullable_schema::<bool>")]
    pub needs_more_thoughts: Option<bool>,
}

/// Output response returned from the `sequentialthinking` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SequentialThinkingResponse {
    /// The thought number that was just processed and recorded in sequence.
    pub thought_number: u32,

    /// The updated estimate of total thoughts needed for this problem.
    pub total_thoughts: u32,

    /// Echoes whether another thought step is expected before completing.
    pub next_thought_needed: bool,

    /// List of all active branch identifiers created during this thinking session.
    pub branches: Vec<String>,

    /// Total number of thoughts recorded in the current session history.
    pub thought_history_length: usize,

    /// True if `totalThoughts` was automatically adjusted to match `thoughtNumber`.
    pub total_thoughts_adjusted: bool,

    /// True if this call replaced/updated an existing recorded thought at this `thoughtNumber`.
    pub replaced_existing: bool,
}

