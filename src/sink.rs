use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::model::SequentialThinkingInput;

/// Sink for thought output events. Implement this to control where
/// thought rendering and diagnostic observations are sent.
pub trait ThoughtSink: Send + Sync {
    /// Called after a thought is successfully processed and recorded.
    fn on_thought(&self, input: &SequentialThinkingInput);

    /// Called for non-fatal diagnostic observations (e.g. large thought numbers, sequence gaps).
    fn on_warning(&self, message: &str);

    /// Called after a counter step is processed.
    fn on_counter(&self, log_line: &str) {
        eprintln!("{}\n", log_line);
    }
}

/// Renders thoughts in a traditional log output style and writes to stderr.
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrThoughtSink;

impl ThoughtSink for StderrThoughtSink {
    fn on_thought(&self, input: &SequentialThinkingInput) {
        let formatted = format_thought(input);
        eprintln!("{}\n", formatted);
    }

    fn on_warning(&self, message: &str) {
        eprintln!("{}", message);
    }

    fn on_counter(&self, log_line: &str) {
        eprintln!("{}\n", log_line);
    }
}

/// Discards all output. Used in unit and protocol tests.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopThoughtSink;

impl ThoughtSink for NoopThoughtSink {
    fn on_thought(&self, _input: &SequentialThinkingInput) {}
    fn on_warning(&self, _message: &str) {}
    fn on_counter(&self, _log_line: &str) {}
}

/// Writes thought output and warnings to an underlying writer (e.g. a log file).
#[derive(Clone)]
pub struct WriterThoughtSink<W: Write + Send + Sync + 'static> {
    writer: Arc<Mutex<W>>,
}

impl<W: Write + Send + Sync + 'static> WriterThoughtSink<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }
}

impl<W: Write + Send + Sync + 'static> ThoughtSink for WriterThoughtSink<W> {
    fn on_thought(&self, input: &SequentialThinkingInput) {
        let formatted = format_thought(input);
        if let Ok(mut w) = self.writer.lock() {
            let _ = writeln!(w, "{}\n", formatted);
        }
    }

    fn on_warning(&self, message: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = writeln!(w, "{}", message);
        }
    }

    fn on_counter(&self, log_line: &str) {
        if let Ok(mut w) = self.writer.lock() {
            let _ = writeln!(w, "{}\n", log_line);
        }
    }
}

/// Format a thought into a clean, traditional log message without width-dependent elements.
pub fn format_thought(thought_data: &SequentialThinkingInput) -> String {
    let header = if thought_data.is_revision.unwrap_or(false) {
        let revises = thought_data
            .revises_thought
            .map(|t| format!(" (revising thought {})", t))
            .unwrap_or_default();
        format!(
            "[Revision] {}/{}{}",
            thought_data.thought_number, thought_data.total_thoughts, revises
        )
    } else if let Some(branch_from) = thought_data.branch_from_thought {
        let branch_id_str = thought_data
            .branch_id
            .as_deref()
            .map(|id| format!(", ID: {}", id))
            .unwrap_or_default();
        format!(
            "[Branch] {}/{} (from thought {}{})",
            thought_data.thought_number,
            thought_data.total_thoughts,
            branch_from,
            branch_id_str
        )
    } else if let Some(branch_id) = &thought_data.branch_id {
        let trimmed = branch_id.trim();
        if !trimmed.is_empty() {
            format!(
                "[Branch] {}/{} (branch: {})",
                thought_data.thought_number,
                thought_data.total_thoughts,
                trimmed
            )
        } else {
            format!(
                "[Thought] {}/{}",
                thought_data.thought_number, thought_data.total_thoughts
            )
        }
    } else {
        format!(
            "[Thought] {}/{}",
            thought_data.thought_number, thought_data.total_thoughts
        )
    };

    let thought = thought_data.thought.trim_end();
    if thought.is_empty() {
        format!("{header}:")
    } else if thought.contains('\n') {
        format!("{header}:\n{thought}")
    } else {
        format!("{header}: {thought}")
    }
}
