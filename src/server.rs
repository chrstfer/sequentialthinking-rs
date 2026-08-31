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
    name = "sequential-thinking-server",
    version = "0.1.0",
    instructions = "Sequential Thinking Server enables dynamic, reflective, and non-linear step-by-step problem-solving. Use the `sequentialthinking` tool to break down complex tasks, plan iteratively, verify hypotheses, revise prior deductions, and branch into alternative exploration paths before finalizing conclusions."
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

    /// Create a new instance with a custom state (e.g. for testing).
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

    /// A detailed tool for dynamic and reflective problem-solving through sequential thoughts.
    /// This tool helps analyze problems through an adaptable thinking process that evolves as understanding deepens.
    /// Each thought can build on, question, revise, or branch from previous insights.
    #[tool(
        name = "sequentialthinking",
        description = r#"A detailed tool for dynamic and reflective problem-solving through sequential thoughts.
This tool helps analyze problems through an adaptable thinking process that evolves as understanding deepens. Each thought can build on, question, revise, or branch from previous insights.

### When to use this tool:
- Decomposing complex problems into manageable steps
- Planning and architecture design with room for iteration and revision
- Exploratory analysis that may require course correction or backtracking
- Multi-step reasoning where the full scope is not initially clear
- Maintaining structured reasoning context across multiple steps
- Filtering noise and focusing on relevant facts per analytical step

### Key capabilities:
- Dynamic thought estimates: `totalThoughts` can be adjusted up or down as you progress
- Revisions: previous thoughts can be questioned, corrected, or refined with `isRevision` and `revisesThought`
- Branching: alternative approaches or hypotheses can be explored in parallel with `branchFromThought` and `branchId`
- Expansion: extra thoughts can be added even after reaching the initial estimate using `needsMoreThoughts`
- Verification: hypotheses can be generated and systematically verified step-by-step

### Guidelines for the model:
1. Start with an initial estimate of `totalThoughts`, but adjust it whenever needed.
2. Formulate explicit hypotheses and verify them in subsequent thoughts.
3. If an earlier deduction was flawed, use `isRevision: true` and specify `revisesThought`.
4. If exploring alternative paths, specify `branchFromThought` and a descriptive `branchId`.
5. Only set `nextThoughtNeeded: false` when the solution is complete, verified, and satisfactory."#,
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
}

