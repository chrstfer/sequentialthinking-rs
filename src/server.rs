use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ContentBlock},
    tool, tool_handler, tool_router,
};
use crate::counter::{CounterInput, CounterOutput, CounterState};
use crate::model::SequentialThinkingInput;
use crate::thinking::SequentialThinkingState;

/// Sequential thinking MCP server.
#[derive(Clone)]
pub struct SequentialThinkingServer {
    state: Arc<Mutex<SequentialThinkingState>>,
    counter_state: Arc<Mutex<CounterState>>,
    tool_router: ToolRouter<Self>,
}

impl Default for SequentialThinkingServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler(
    router = self.tool_router,
    name = "seq",
    version = "0.2.0",
    instructions = "Seq Server: provides the thinking tool for iterative reasoning and the count tool for tracking progress across iterative tasks."
)]
impl ServerHandler for SequentialThinkingServer {}

#[tool_router(router = tool_router)]
impl SequentialThinkingServer {
    /// Create a new instance of the SequentialThinkingServer.
    pub fn new() -> Self {
        Self::with_states(
            SequentialThinkingState::new(),
            CounterState::new(),
        )
    }

    /// Create a new instance with a custom thinking state.
    pub fn with_state(state: SequentialThinkingState) -> Self {
        Self::with_states(
            state,
            CounterState::new(),
        )
    }

    /// Create a new instance with custom thinking and counter states.
    pub fn with_states(
        thinking_state: SequentialThinkingState,
        counter_state: CounterState,
    ) -> Self {
        Self {
            state: Arc::new(Mutex::new(thinking_state)),
            counter_state: Arc::new(Mutex::new(counter_state)),
            tool_router: Self::tool_router(),
        }
    }

    /// Access the underlying thinking state.
    pub fn state(&self) -> Arc<Mutex<SequentialThinkingState>> {
        Arc::clone(&self.state)
    }

    /// Access the underlying counter state.
    pub fn counter_state(&self) -> Arc<Mutex<CounterState>> {
        Arc::clone(&self.counter_state)
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
        name = "thinking",
        description = r#"Step-by-step reasoning engine supporting revisions and branching exploration.

Record one thought per call. Thoughts can revise previous thoughts (`isRevision: true`, `revisesThought`) or branch into alternative exploration paths (`branchFromThought`, `branchId`). Always pass `nextThoughtNeeded: true` while reasoning is ongoing; set `nextThoughtNeeded: false` only when the solution is reached."#,
        annotations(
            title = "Thinking",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    pub async fn thinking(
        &self,
        params: Parameters<SequentialThinkingInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        self.process_thought_impl(params).await
    }
    /// Track progress across iterative tasks. Call with total to initialize; call with no args to step.
    #[tool(
        name = "count",
        description = "Track progress across iterative tasks. Call with total to initialize; call with no args to step.",
        annotations(
            title = "Task Counter",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    pub async fn count(
        &self,
        params: Parameters<CounterInput>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut state = self.counter_state.lock().await;
        match state.process(params.0) {
            Ok(CounterOutput::Progress(resp)) => {
                let json_text =
                    serde_json::to_string(&resp).unwrap_or_else(|_| "{}".to_string());
                Ok(CallToolResult::success(vec![ContentBlock::text(json_text)]))
            }
            Ok(CounterOutput::Finished(msg)) => {
                Ok(CallToolResult::success(vec![ContentBlock::text(msg)]))
            }
            Err(err_msg) => Ok(CallToolResult::error(vec![ContentBlock::text(err_msg)])),
        }
    }
}
