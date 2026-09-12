use std::collections::HashMap;

use crate::model::{SequentialThinkingInput, SequentialThinkingResponse};
use crate::sink::{StderrThoughtSink, ThoughtSink};

/// State machine managing sequential thinking history and active branches.
pub struct SequentialThinkingState {
    thought_history: Vec<SequentialThinkingInput>,
    branches: HashMap<String, Vec<SequentialThinkingInput>>,
    sink: Box<dyn ThoughtSink>,
}

impl std::fmt::Debug for SequentialThinkingState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SequentialThinkingState")
            .field("thought_history", &self.thought_history)
            .field("branches", &self.branches)
            .field("sink", &"<ThoughtSink>")
            .finish()
    }
}

impl Default for SequentialThinkingState {
    fn default() -> Self {
        Self::new()
    }
}

impl SequentialThinkingState {
    /// Create a new thinking state instance using the default StderrThoughtSink.
    pub fn new() -> Self {
        Self::with_sink(Box::new(StderrThoughtSink))
    }

    /// Create a new thinking state instance with a custom thought sink.
    pub fn with_sink(sink: Box<dyn ThoughtSink>) -> Self {
        Self {
            thought_history: Vec::new(),
            branches: HashMap::new(),
            sink,
        }
    }

    /// Process a new thought step, update internal history/branches, and emit output through the sink.
    pub fn process_thought(
        &mut self,
        mut input: SequentialThinkingInput,
    ) -> Result<SequentialThinkingResponse, String> {
        // Validate thought text
        if input.thought.trim().is_empty() {
            return Err("Thinking Error:\n\
                 - Field: `thought`\n\
                 - Received: empty content\n\
                 - Constraint: `thought` must contain substantive reasoning, analysis, or hypothesis text.\n\
                 - Suggestion: Provide your analytical reasoning or deduction for this step in the `thought` field.".to_string());
        }

        // Validate thoughtNumber
        if input.thought_number == 0 {
            return Err("Thinking Error:\n\
                 - Field: `thoughtNumber`\n\
                 - Received: 0\n\
                 - Constraint: `thoughtNumber` must be an integer >= 1.\n\
                 - Suggestion: Thoughts are 1-based. Please start with `thoughtNumber: 1` for the initial step.".to_string());
        }

        // Validate totalThoughts
        if input.total_thoughts == 0 {
            return Err("Thinking Error:\n\
                 - Field: `totalThoughts`\n\
                 - Received: 0\n\
                 - Constraint: `totalThoughts` must be an integer >= 1.\n\
                 - Suggestion: Provide an estimated total number of thoughts (e.g. 3, 5, 10). This can be adjusted dynamically.".to_string());
        }

        // Check for unusually large thought numbers or total thoughts
        if input.thought_number > 1000 || input.total_thoughts > 1000 {
            self.sink.on_warning(&format!(
                "Warning: `thoughtNumber` ({}) or `totalThoughts` ({}) is unusually large (> 1000).",
                input.thought_number, input.total_thoughts
            ));
        }

        // Note sequence gaps
        let max_existing_thought = self
            .thought_history
            .iter()
            .map(|t| t.thought_number)
            .max()
            .unwrap_or(0);
        if !self.thought_history.is_empty() && input.thought_number > max_existing_thought + 1 {
            self.sink.on_warning(&format!(
                "Note: Sequence gap detected: `thoughtNumber` {} follows max prior thought {}.",
                input.thought_number, max_existing_thought
            ));
        }

        // Validate revisesThought consistency with isRevision (M-1 & M-2)
        if input.is_revision == Some(true) && input.revises_thought.is_none() {
            return Err("Thinking Error:\n\
                 - Field: `revisesThought`\n\
                 - Received: missing\n\
                 - Constraint: `revisesThought` is required when `isRevision` is true.\n\
                 - Suggestion: Specify the 1-based thought number from history being revised.".to_string());
        }

        if input.revises_thought.is_some() && input.is_revision != Some(true) {
            return Err("Thinking Error:\n\
                 - Field: `isRevision`\n\
                 - Received: false or null\n\
                 - Constraint: `isRevision` must be true when `revisesThought` is specified.\n\
                 - Suggestion: Set `isRevision: true` when referencing a previous thought to revise.".to_string());
        }

        // Validate revisesThought bounds if specified
        if let Some(rev) = input.revises_thought {
            if rev == 0 {
                return Err("Thinking Error:\n\
                     - Field: `revisesThought`\n\
                     - Received: 0\n\
                     - Constraint: `revisesThought` must be a 1-based thought number (>= 1).\n\
                     - Suggestion: Specify the 1-based thought number from history that is being revised.".to_string());
            }
            if rev > input.thought_number {
                return Err(format!(
                    "Thinking Error:\n\
                     - Field: `revisesThought`\n\
                     - Received: {}\n\
                     - Constraint: `revisesThought` ({}) cannot exceed current `thoughtNumber` ({}).\n\
                     - Suggestion: You can only revise thoughts that occurred prior to or at the current step.",
                    rev, rev, input.thought_number
                ));
            }
            if !self.thought_history.iter().any(|t| t.thought_number == rev) {
                return Err(format!(
                    "Thinking Error:\n\
                     - Field: `revisesThought`\n\
                     - Received: {}\n\
                     - Constraint: `revisesThought` must reference a thought number that exists in session history.\n\
                     - Suggestion: Choose a thought number from existing history to revise.",
                    rev
                ));
            }
        }

        // Validate branchFromThought and branchId if specified
        if let Some(branch_from) = input.branch_from_thought {
            if branch_from == 0 {
                return Err("Thinking Error:\n\
                     - Field: `branchFromThought`\n\
                     - Received: 0\n\
                     - Constraint: `branchFromThought` must be a 1-based thought number (>= 1).\n\
                     - Suggestion: Specify a valid 1-based thought number from history as the branching origin.".to_string());
            }
            if input.branch_id.as_deref().unwrap_or("").trim().is_empty() {
                return Err("Thinking Error:\n\
                     - Field: `branchId`\n\
                     - Received: missing or empty\n\
                     - Constraint: `branchId` is required when `branchFromThought` is specified.\n\
                     - Suggestion: Provide a descriptive string identifier for the branch (e.g. 'approach-b', 'hypothesis-2').".to_string());
            }
            if branch_from > input.thought_number {
                return Err(format!(
                    "Thinking Error:\n\
                     - Field: `branchFromThought`\n\
                     - Received: {}\n\
                     - Constraint: `branchFromThought` ({}) cannot exceed current `thoughtNumber` ({}).\n\
                     - Suggestion: You can only branch from thoughts that occurred prior to or at the current step.",
                    branch_from, branch_from, input.thought_number
                ));
            }
            if !self.thought_history.iter().any(|t| t.thought_number == branch_from) {
                return Err(format!(
                    "Thinking Error:\n\
                     - Field: `branchFromThought`\n\
                     - Received: {}\n\
                     - Constraint: `branchFromThought` must reference a thought number that exists in session history.\n\
                     - Suggestion: Choose a valid origin thought number from existing history.",
                    branch_from
                ));
            }
        }

        // Adjust total_thoughts dynamically if current thought_number exceeds estimate
        let total_thoughts_adjusted = input.thought_number > input.total_thoughts;
        if total_thoughts_adjusted {
            input.total_thoughts = input.thought_number;
        }

        // Idempotency / Upsert: If this thought_number was already recorded,
        // update the recorded thought in-place and clean up any orphaned old branch.
        if let Some(pos) = self
            .thought_history
            .iter()
            .position(|t| t.thought_number == input.thought_number)
        {
            let old_thought = self.thought_history[pos].clone();
            self.thought_history[pos] = input.clone();

            let old_branch = old_thought
                .branch_id
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
            let new_branch = input
                .branch_id
                .as_deref()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());

            // If old thought belonged to a different branch, remove from old branch
            if old_branch != new_branch {
                if let Some(old_b_id) = old_branch {
                    if let Some(branch_vec) = self.branches.get_mut(old_b_id) {
                        branch_vec.retain(|t| t.thought_number != input.thought_number);
                        if branch_vec.is_empty() {
                            self.branches.remove(old_b_id);
                        }
                    }
                }
            }

            // Update or insert into new branch
            if let Some(b_id) = new_branch {
                let branch_vec = self.branches.entry(b_id.to_string()).or_default();
                if let Some(b_pos) = branch_vec
                    .iter()
                    .position(|t| t.thought_number == input.thought_number)
                {
                    branch_vec[b_pos] = input.clone();
                } else {
                    branch_vec.push(input.clone());
                }
            }

            self.sink.on_thought(&input);

            let mut branches: Vec<String> = self.branches.keys().cloned().collect();
            branches.sort();

            return Ok(SequentialThinkingResponse {
                thought_number: input.thought_number,
                total_thoughts: input.total_thoughts,
                next_thought_needed: input.next_thought_needed,
                branches,
                thought_history_length: self.thought_history.len(),
                total_thoughts_adjusted,
                replaced_existing: true,
            });
        }

        // Record branch history when branch_id is specified (includes both origins and continuations)
        if let Some(branch_id) = &input.branch_id {
            let trimmed = branch_id.trim();
            if !trimmed.is_empty() {
                self.branches
                    .entry(trimmed.to_string())
                    .or_default()
                    .push(input.clone());
            }
        }

        self.sink.on_thought(&input);

        let thought_number = input.thought_number;
        let total_thoughts = input.total_thoughts;
        let next_thought_needed = input.next_thought_needed;

        self.thought_history.push(input);

        let mut branches: Vec<String> = self.branches.keys().cloned().collect();
        branches.sort();

        Ok(SequentialThinkingResponse {
            thought_number,
            total_thoughts,
            next_thought_needed,
            branches,
            thought_history_length: self.thought_history.len(),
            total_thoughts_adjusted,
            replaced_existing: false,
        })
    }

    /// Retrieve the current thought history.
    pub fn history(&self) -> &[SequentialThinkingInput] {
        &self.thought_history
    }

    /// Retrieve all branches.
    pub fn branches(&self) -> &HashMap<String, Vec<SequentialThinkingInput>> {
        &self.branches
    }
}
