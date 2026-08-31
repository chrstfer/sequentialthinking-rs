use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use sequentialthinking_rs::SequentialThinkingServer;
use sequentialthinking_rs::model::{SequentialThinkingInput, SequentialThinkingResponse};

#[tokio::test]
async fn test_tool_execution_via_server() {
    let server = SequentialThinkingServer::new();
    {
        let state_arc = server.state();
        let mut state = state_arc.lock().await;
        state.set_disable_thought_logging(true);
    }

    let input1 = SequentialThinkingInput {
        thought: "Testing tool handler execution - thought 1".to_string(),
        next_thought_needed: true,
        thought_number: 1,
        total_thoughts: 2,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let result1 = server.sequentialthinking(Parameters(input1)).await.unwrap();
    assert_eq!(result1.is_error, Some(false));
    let text1 = result1.content[0].as_text().unwrap();
    let resp1: SequentialThinkingResponse = serde_json::from_str(&text1.text).unwrap();
    assert_eq!(resp1.thought_number, 1);
    assert_eq!(resp1.total_thoughts, 2);
    assert!(resp1.next_thought_needed);
    assert_eq!(resp1.thought_history_length, 1);

    let input2 = SequentialThinkingInput {
        thought: "Testing tool handler execution - thought 2".to_string(),
        next_thought_needed: false,
        thought_number: 2,
        total_thoughts: 2,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let result2 = server.sequentialthinking(Parameters(input2)).await.unwrap();
    assert_eq!(result2.is_error, Some(false));
    let text2 = result2.content[0].as_text().unwrap();
    let resp2: SequentialThinkingResponse = serde_json::from_str(&text2.text).unwrap();
    assert_eq!(resp2.thought_number, 2);
    assert_eq!(resp2.total_thoughts, 2);
    assert!(!resp2.next_thought_needed);
    assert_eq!(resp2.thought_history_length, 2);
}

#[tokio::test]
async fn test_tool_error_via_server() {
    let server = SequentialThinkingServer::new();
    {
        let state_arc = server.state();
        let mut state = state_arc.lock().await;
        state.set_disable_thought_logging(true);
    }

    let invalid_input = SequentialThinkingInput {
        thought: "Testing error handling".to_string(),
        next_thought_needed: true,
        thought_number: 0,
        total_thoughts: 2,
        is_revision: None,
        revises_thought: None,
        branch_from_thought: None,
        branch_id: None,
        needs_more_thoughts: None,
    };

    let result = server.sequentialthinking(Parameters(invalid_input)).await.unwrap();
    assert_eq!(result.is_error, Some(true));
    let text = result.content[0].as_text().unwrap();
    assert!(text.text.contains("Sequential Thinking Error:"));
    assert!(text.text.contains("- Field: `thoughtNumber`"));
    assert!(text.text.contains("- Constraint:"));
    assert!(text.text.contains("- Suggestion:"));
    assert!(text.text.contains('\n'));
}

#[tokio::test]
async fn test_server_info_and_instructions() {
    let server = SequentialThinkingServer::new();
    let info = server.get_info();

    assert_eq!(info.server_info.name, "sequential-thinking-server");
    assert_eq!(info.server_info.version, "0.1.0");
    assert!(info.instructions.is_some());
    let instructions = info.instructions.unwrap();
    assert!(instructions.contains("Sequential Thinking Server"));
    assert!(instructions.contains("sequentialthinking"));
}

