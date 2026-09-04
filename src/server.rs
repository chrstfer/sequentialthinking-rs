use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    tool, tool_handler, tool_router,
};

use crate::model::SequentialThinkingInput;
use crate::thinking::SequentialThinkingState;

/// Sequential thinking MCP server.
#[derive(Clone)]
pub struct SequentialThinkingServer {
    state: Arc<Mutex<SequentialThinkingState>>,
    tool_router: ToolRouter<Self>,
}

impl Default for SequentialThinkingServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(
    router = self.tool_router,
    name = "sequentialthinking",
    version = "0.1.0",
    instructions = "Sequential Thinking Server: provides the sequentialthinking tool for iterative, non-linear reasoning with branching and revisions."
)]
impl ServerHandler for SequentialThinkingServer {}

#[tool_router(router = tool_router)]
impl SequentialThinkingServer {
    /// Create a new instance of the SequentialThinkingServer.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(SequentialThinkingState::new())),
            tool_router: Self::tool_router(),
        }
    }

    /// Create a new instance with a custom state (e.g. for testing with custom sinks).
    pub fn with_state(state: SequentialThinkingState) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
            tool_router: Self::tool_router(),
        }
    }

    /// Access the underlying state.
    pub fn state(&self) -> Arc<Mutex<SequentialThinkingState>> {
        Arc::clone(&self.state)
    }

    /// Internal helper to execute thought processing and format results.
    async fn process_thought_impl(
        &self,
        params: Parameters<SequentialThinkingInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut state = self.state.lock().await;
        match state.process_thought(params.0) {
            Ok(response) => {
                let json_text = serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| "{}".to_string());
                Ok(CallToolResult::success(vec![ContentBlock::text(json_text)]))
            }
            Err(err_msg) => {
                Ok(CallToolResult::error(vec![ContentBlock::text(err_msg)]))
            }
        }
    }

    /// Step-by-step reasoning engine supporting revisions and branching exploration.
    #[tool(
        name = "sequentialthinking",
        description = r#"Step-by-step reasoning engine supporting revisions and branching exploration.

Record one thought per call. Thoughts can revise previous thoughts (`isRevision: true`, `revisesThought`) or branch into alternative exploration paths (`branchFromThought`, `branchId`). Set `nextThoughtNeeded: false` when the problem is resolved."#,
        annotations(
            title = "Sequential Thinking",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn sequentialthinking(
        &self,
        params: Parameters<SequentialThinkingInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.process_thought_impl(params).await
    }

    /// Step-by-step reasoning engine supporting revisions and branching exploration. (Alias for `sequentialthinking`).
    #[tool(
        name = "sequentialthinking-rs",
        description = r#"Step-by-step reasoning engine supporting revisions and branching exploration. (Alias for sequentialthinking)."#,
        annotations(
            title = "Sequential Thinking",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn sequentialthinking_rs(
        &self,
        params: Parameters<SequentialThinkingInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.process_thought_impl(params).await
    }
}
