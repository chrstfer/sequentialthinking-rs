use sequentialthinking_rs::model::{SequentialThinkingInput, SequentialThinkingResponse};
use sequentialthinking_rs::sink::{NoopThoughtSink, format_thought};
use sequentialthinking_rs::thinking::SequentialThinkingState;

#[test]
fn test_schema_portability_and_structure() {
    let schema = schemars::schema_for!(SequentialThinkingInput);
    let json_val = serde_json::to_value(&schema).unwrap();

    // Check additionalProperties: false
    assert_eq!(
        json_val.get("additionalProperties"),
        Some(&serde_json::Value::Bool(false)),
        "Schema must set additionalProperties to false"
    );

    let properties = json_val["properties"].as_object().expect("properties object");

    // Required fields check
    let required: Vec<&str> = json_val["required"]
        .as_array()
        .expect("required array")
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();

    assert_eq!(
        required,
        vec!["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
    );

    // thoughtNumber and totalThoughts must have minimum: 1
    assert_eq!(
        properties["thoughtNumber"]["minimum"],
        serde_json::json!(1),
        "thoughtNumber minimum must be 1"
    );
    assert_eq!(
        properties["totalThoughts"]["minimum"],
        serde_json::json!(1),
        "totalThoughts minimum must be 1"
    );

    // All 5 nullable fields must use `anyOf` with single-type branches, never array `type`
    let nullable_fields = [
        ("isRevision", "boolean"),
        ("revisesThought", "integer"),
        ("branchFromThought", "integer"),
        ("branchId", "string"),
        ("needsMoreThoughts", "boolean"),
    ];

    for (field, expected_type) in nullable_fields {
        let prop = &properties[field];
        assert!(
            prop.get("type").is_none(),
            "Field {} must not have a top-level type array",
            field
        );

        let any_of = prop["anyOf"]
            .as_array()
            .unwrap_or_else(|| panic!("Field {} must contain anyOf array", field));
        assert_eq!(any_of.len(), 2, "Field {} anyOf must have 2 branches", field);

        let type_branch = any_of
            .iter()
            .find(|b| b["type"] == expected_type)
            .unwrap_or_else(|| panic!("Field {} anyOf must contain type: {}", field, expected_type));
        let has_null_type = any_of.iter().any(|b| b["type"] == "null");

        assert!(
            has_null_type,
            "Field {} anyOf must contain type: null",
            field
        );

        // For revisesThought and branchFromThought, integer branch must have minimum: 1
        if field == "revisesThought" || field == "branchFromThought" {
            assert_eq!(
                type_branch["minimum"],
                serde_json::json!(1),
                "Field {} integer branch in anyOf must specify minimum: 1",
                field
            );
        }
    }
}

#[test]
fn test_basic_thought_progression() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    let t1 = SequentialThinkingInput {
        thought: "First step: analyzing problem".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let resp1 = state.process_thought(t1).unwrap();
    assert_eq!(
        resp1,
        SequentialThinkingResponse {
            thought_number: 1,
            total_thoughts: 3,
            next_thought_needed: true,
            branches: vec![],
            thought_history_length: 1,
            total_thoughts_adjusted: false,
            replaced_existing: false,
        }
    );

    let t2 = SequentialThinkingInput {
        thought: "Second step: formulating hypothesis".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let resp2 = state.process_thought(t2).unwrap();
    assert_eq!(resp2.thought_number, 2);
    assert_eq!(resp2.thought_history_length, 2);

    let t3 = SequentialThinkingInput {
        thought: "Third step: verified final conclusion".to_string(),
        next_thought_needed: false,
        thought_number: 3,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let resp3 = state.process_thought(t3).unwrap();
    assert_eq!(resp3.thought_number, 3);
    assert!(!resp3.next_thought_needed);
    assert_eq!(resp3.thought_history_length, 3);
    assert_eq!(state.history().len(), 3);
}

#[test]
fn test_dynamic_total_thoughts_adjustment() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    let t = SequentialThinkingInput {
        thought: "Need an extra step beyond initial plan".to_string(),
        next_thought_needed: true,
        thought_number: 5,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: Some(true),
    };

    let resp = state.process_thought(t).unwrap();
    // total_thoughts should be dynamically adjusted to at least thought_number
    assert_eq!(resp.thought_number, 5);
    assert_eq!(resp.total_thoughts, 5);
}

#[test]
fn test_revision_and_formatting() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    let t1 = SequentialThinkingInput {
        thought: "Initial premise".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 4,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    state.process_thought(t1).unwrap();

    let t2 = SequentialThinkingInput {
        thought: "Revising our previous assumption about data size".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 4,
        is_revision: Some(true),
        revises_thought: Some(1),
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let formatted = format_thought(&t2);
    assert!(formatted.contains("[Revision] 2/4 (revising thought 1)"));
    assert!(formatted.contains("Revising our previous assumption about data size"));
    assert!(formatted.starts_with('+'));
    assert!(formatted.ends_with('+'));

    let resp = state.process_thought(t2).unwrap();
    assert_eq!(resp.thought_number, 2);
}

#[test]
fn test_branching_and_continuations() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    let t1 = SequentialThinkingInput {
        thought: "Root thought for branching".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 4,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    state.process_thought(t1).unwrap();

    // Branch origin thought (branchFromThought + branchId)
    let t_branch_a1 = SequentialThinkingInput {
        thought: "Exploring branch approach A - step 1".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 4,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: Some(1),
        branch_id: Some("branch-a".to_string()),
        needs_more_thoughts: None,
    };

    let formatted_branch = format_thought(&t_branch_a1);
    assert!(formatted_branch.contains("[Branch] 2/4 (from thought 1, ID: branch-a)"));

    let resp_a1 = state.process_thought(t_branch_a1).unwrap();
    assert_eq!(resp_a1.branches, vec!["branch-a".to_string()]);
    assert_eq!(state.branches()["branch-a"].len(), 1);

    // Branch continuation thought (branchId without branchFromThought)
    let t_branch_a2 = SequentialThinkingInput {
        thought: "Continuing branch approach A - step 2".to_string(),
        next_thought_needed: true,
        thought_number: 3,
        total_thoughts: 4,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: Some("branch-a".to_string()),
        needs_more_thoughts: None,
    };

    let formatted_a2 = format_thought(&t_branch_a2);
    assert!(formatted_a2.contains("[Branch] 3/4 (branch: branch-a)"));

    let resp_a2 = state.process_thought(t_branch_a2).unwrap();
    assert_eq!(resp_a2.branches, vec!["branch-a".to_string()]);
    assert_eq!(state.branches()["branch-a"].len(), 2);

    // Second branch origin (branch-b)
    let t_branch_b = SequentialThinkingInput {
        thought: "Exploring branch approach B".to_string(),
        next_thought_needed: true,
        thought_number: 4,
        total_thoughts: 4,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: Some(1),
        branch_id: Some("branch-b".to_string()),
        needs_more_thoughts: None,
    };

    let resp_b = state.process_thought(t_branch_b).unwrap();
    assert_eq!(
        resp_b.branches,
        vec!["branch-a".to_string(), "branch-b".to_string()]
    );
    assert_eq!(state.branches().len(), 2);
}

#[test]
fn test_multiline_ascii_formatting() {
    let t = SequentialThinkingInput {
        thought: "Line one of thought\nLine two with more detail\nLine three conclusion"
            .to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let formatted = format_thought(&t);
    assert!(formatted.contains("[Thought] 1/5"));
    assert!(formatted.contains("Line one of thought"));
    assert!(formatted.contains("Line two with more detail"));
    assert!(formatted.contains("Line three conclusion"));

    // Verify all lines have ASCII box borders
    for line in formatted.lines() {
        assert!(
            (line.starts_with('+') && line.ends_with('+'))
                || (line.starts_with('|') && line.ends_with('|'))
        );
    }
}

#[test]
fn test_validation_errors_and_formatting() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    // Empty thought
    let empty_thought = SequentialThinkingInput {
        thought: "   ".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(empty_thought).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `thought`"));
    assert!(err.contains('\n'));

    // thoughtNumber 0
    let invalid_num = SequentialThinkingInput {
        thought: "test".to_string(),
        next_thought_needed: true,
        thought_number: 0,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(invalid_num).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `thoughtNumber`"));
    assert!(err.contains("- Suggestion:"));
    assert!(err.contains('\n'));

    // totalThoughts 0
    let invalid_total = SequentialThinkingInput {
        thought: "test".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 0,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(invalid_total).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `totalThoughts`"));
    assert!(err.contains('\n'));

    // isRevision: true without revisesThought (M-1)
    let rev_without_target = SequentialThinkingInput {
        thought: "test revision".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 5,
        is_revision: Some(true),
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(rev_without_target).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `revisesThought`"));
    assert!(err.contains("is required when `isRevision` is true"));

    // revisesThought without isRevision: true (M-2)
    let target_without_rev = SequentialThinkingInput {
        thought: "test revision".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: Some(1),
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(target_without_rev).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `isRevision`"));
    assert!(err.contains("must be true when `revisesThought` is specified"));

    // Populate a thought in history
    let t1 = SequentialThinkingInput {
        thought: "base thought".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    state.process_thought(t1).unwrap();

    // Revising a thought that doesn't exist in history yet
    let invalid_rev = SequentialThinkingInput {
        thought: "test revision".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 5,
        is_revision: Some(true),
        revises_thought: Some(3),
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(invalid_rev).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `revisesThought`"));
    assert!(err.contains('\n'));

    // Branch without branchId
    let invalid_branch = SequentialThinkingInput {
        thought: "test branch".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: Some(1),
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(invalid_branch).unwrap_err();
    assert!(err.contains("Sequential Thinking Error:"));
    assert!(err.contains("- Field: `branchId`"));
    assert!(err.contains('\n'));
}

#[test]
fn test_serde_json_compatibility() {
    // CamelCase JSON input (standard MCP client)
    let json_camel = r#"{
        "thought": "CamelCase test",
        "nextThoughtNeeded": true,
        "thoughtNumber": 1,
        "totalThoughts": 4,
        "isRevision": false,
        "revisesThought": 1,
        "branchFromThought": 1,
        "branchId": "test-branch",
        "needsMoreThoughts": false
    }"#;

    let parsed_camel: SequentialThinkingInput = serde_json::from_str(json_camel).unwrap();
    assert_eq!(parsed_camel.thought, "CamelCase test");
    assert_eq!(parsed_camel.thought_number, 1);
    assert_eq!(parsed_camel.total_thoughts, 4);
    assert_eq!(parsed_camel.branch_id.as_deref(), Some("test-branch"));

    // Snake_case JSON input (fallback compatibility)
    let json_snake = r#"{
        "thought": "SnakeCase test",
        "next_thought_needed": false,
        "thought_number": 2,
        "total_thoughts": 5,
        "is_revision": true,
        "revises_thought": 1
    }"#;

    let parsed_snake: SequentialThinkingInput = serde_json::from_str(json_snake).unwrap();
    assert_eq!(parsed_snake.thought, "SnakeCase test");
    assert_eq!(parsed_snake.thought_number, 2);
    assert!(!parsed_snake.next_thought_needed);
    assert_eq!(parsed_snake.is_revision, Some(true));
    assert_eq!(parsed_snake.revises_thought, Some(1));

    // Response serialization generates camelCase
    let resp = SequentialThinkingResponse {
        thought_number: 1,
        total_thoughts: 3,
        next_thought_needed: true,
        branches: vec!["b1".to_string()],
        thought_history_length: 1,
        total_thoughts_adjusted: false,
        replaced_existing: false,
    };

    let serialized = serde_json::to_string(&resp).unwrap();
    assert!(serialized.contains(r#""thoughtNumber":1"#));
    assert!(serialized.contains(r#""totalThoughts":3"#));
    assert!(serialized.contains(r#""nextThoughtNeeded":true"#));
    assert!(serialized.contains(r#""thoughtHistoryLength":1"#));
    assert!(serialized.contains(r#""totalThoughtsAdjusted":false"#));
    assert!(serialized.contains(r#""replacedExisting":false"#));
}

#[test]
fn test_idempotent_thought_deduplication_and_branch_cleanup() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    let t1 = SequentialThinkingInput {
        thought: "Initial analysis step".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    // First call: records thought
    let resp1 = state.process_thought(t1.clone()).unwrap();
    assert_eq!(resp1.thought_number, 1);
    assert_eq!(resp1.thought_history_length, 1);
    assert!(!resp1.replaced_existing);
    assert_eq!(state.history().len(), 1);
    assert_eq!(state.history()[0].thought, "Initial analysis step");

    // Exact duplicate call: returns identical result with replaced_existing: true
    let resp1_dup = state.process_thought(t1.clone()).unwrap();
    assert_eq!(resp1_dup.thought_number, 1);
    assert_eq!(resp1_dup.thought_history_length, 1);
    assert!(resp1_dup.replaced_existing);
    assert_eq!(state.history().len(), 1);

    // Stochastic / reworded retry of thought 1 (e.g. after timeout)
    let t1_reworded = SequentialThinkingInput {
        thought: "Initial analysis step with slightly reworded phrasing".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let resp1_reworded = state.process_thought(t1_reworded).unwrap();
    assert_eq!(resp1_reworded.thought_number, 1);
    assert_eq!(resp1_reworded.thought_history_length, 1);
    assert!(resp1_reworded.replaced_existing);
    assert_eq!(state.history().len(), 1);
    assert_eq!(
        state.history()[0].thought,
        "Initial analysis step with slightly reworded phrasing"
    );

    let t2 = SequentialThinkingInput {
        thought: "Second step creating branch A".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: Some(1),
        branch_id: Some("branch-a".to_string()),
        needs_more_thoughts: None,
    };

    // Second thought: records thought and branch-a
    let resp2 = state.process_thought(t2.clone()).unwrap();
    assert_eq!(resp2.thought_number, 2);
    assert_eq!(resp2.thought_history_length, 2);
    assert!(!resp2.replaced_existing);
    assert_eq!(resp2.branches, vec!["branch-a".to_string()]);
    assert_eq!(state.branches()["branch-a"].len(), 1);

    // Reworded retry changing branch to branch-b (branch-a should be cleaned up!)
    let t2_reworded_b = SequentialThinkingInput {
        thought: "Second step changing branch to B".to_string(),
        next_thought_needed: true,
        thought_number: 2,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: Some(1),
        branch_id: Some("branch-b".to_string()),
        needs_more_thoughts: None,
    };
    let resp2_reworded_b = state.process_thought(t2_reworded_b).unwrap();
    assert_eq!(resp2_reworded_b.thought_number, 2);
    assert_eq!(resp2_reworded_b.thought_history_length, 2);
    assert!(resp2_reworded_b.replaced_existing);
    // branch-a was cleaned up, now only branch-b remains
    assert_eq!(resp2_reworded_b.branches, vec!["branch-b".to_string()]);
    assert!(!state.branches().contains_key("branch-a"));
    assert_eq!(state.branches()["branch-b"].len(), 1);
}

#[test]
fn test_total_thoughts_adjusted_flag() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    let t = SequentialThinkingInput {
        thought: "Need an extra step beyond initial plan".to_string(),
        next_thought_needed: true,
        thought_number: 5,
        total_thoughts: 3,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: Some(true),
    };

    let resp = state.process_thought(t).unwrap();
    assert_eq!(resp.thought_number, 5);
    assert_eq!(resp.total_thoughts, 5);
    assert!(resp.total_thoughts_adjusted);
    assert!(!resp.replaced_existing);
}

#[test]
fn test_non_dense_thought_numbers_and_revision_validation() {
    let mut state = SequentialThinkingState::with_sink(Box::new(NoopThoughtSink));

    // Thought 1
    let t1 = SequentialThinkingInput {
        thought: "Thought 1".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    state.process_thought(t1).unwrap();

    // Thought 3 (gap skipping 2)
    let t3 = SequentialThinkingInput {
        thought: "Thought 3".to_string(),
        next_thought_needed: true,
        thought_number: 3,
        total_thoughts: 5,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    state.process_thought(t3).unwrap();
    assert_eq!(state.history().len(), 2);

    // Revising thought 2 (which does not exist in history) should fail even though 2 <= history.len()
    let rev2 = SequentialThinkingInput {
        thought: "Revising non-existent thought 2".to_string(),
        next_thought_needed: true,
        thought_number: 4,
        total_thoughts: 5,
        is_revision: Some(true),
        revises_thought: Some(2),
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let err = state.process_thought(rev2).unwrap_err();
    assert!(err.contains("must reference a thought number that exists in session history"));

    // Revising thought 1 (which does exist) succeeds
    let rev1 = SequentialThinkingInput {
        thought: "Revising existent thought 1".to_string(),
        next_thought_needed: true,
        thought_number: 4,
        total_thoughts: 5,
        is_revision: Some(true),
        revises_thought: Some(1),
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };
    let resp = state.process_thought(rev1).unwrap();
    assert_eq!(resp.thought_number, 4);
    assert_eq!(resp.thought_history_length, 3);
}


