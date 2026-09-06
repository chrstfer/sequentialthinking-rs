use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::sink::{StderrThoughtSink, ThoughtSink};

/// Input parameters for the `counter` tool.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterInput {
    /// Counter name (defaults to "default").
    #[serde(default)]
    pub name: Option<String>,

    /// Set or update total count.
    #[serde(default)]
    pub total: Option<u32>,

    /// Explicitly set current count.
    #[serde(default)]
    pub current: Option<u32>,

    /// Increment amount (default: 1; set 0 to query status without stepping).
    #[serde(default)]
    pub step: Option<u32>,

    /// Reset counter before applying step.
    #[serde(default)]
    pub reset: Option<bool>,
}

/// Output response returned from the `counter` tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CounterResponse {
    /// Counter identifier name.
    pub name: String,

    /// Current progress count.
    pub current: u32,

    /// Total target count if set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,

    /// Steps remaining until total is reached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u32>,

    /// Whether the counter has reached or exceeded total.
    pub done: bool,
}

/// State tracking a single named counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCounter {
    pub current: u32,
    pub total: Option<u32>,
    pub done: bool,
}

/// Result of processing a counter step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CounterOutput {
    /// Active counter progress information.
    Progress(CounterResponse),

    /// Completion notice when a step is attempted on an already finished counter.
    Finished(String),
}

/// State manager for task counters.
pub struct CounterState {
    counters: HashMap<String, TaskCounter>,
    sink: Box<dyn ThoughtSink>,
}

impl std::fmt::Debug for CounterState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterState")
            .field("counters", &self.counters)
            .field("sink", &"<ThoughtSink>")
            .finish()
    }
}

impl Default for CounterState {
    fn default() -> Self {
        Self::new()
    }
}

impl CounterState {
    /// Create a new counter state instance with default stderr sink.
    pub fn new() -> Self {
        Self::with_sink(Box::new(StderrThoughtSink))
    }

    /// Create a new counter state with a custom sink.
    pub fn with_sink(sink: Box<dyn ThoughtSink>) -> Self {
        Self {
            counters: HashMap::new(),
            sink,
        }
    }

    /// Access all tracked counters.
    pub fn counters(&self) -> &HashMap<String, TaskCounter> {
        &self.counters
    }

    /// Process a counter request, updating internal state and emitting log output.
    pub fn process(&mut self, input: CounterInput) -> Result<CounterOutput, String> {
        let name = input
            .name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("default")
            .to_string();

        if input.reset == Some(true) {
            self.counters.remove(&name);
        }

        let existing = self.counters.get_mut(&name);
        match existing {
            Some(counter) => {
                if counter.done && input.total.is_none() && input.current.is_none() {
                    return Ok(CounterOutput::Finished(
                        "Your counter has finished, move on.".to_string(),
                    ));
                }

                if input.total.is_some() || input.current.is_some() {
                    counter.done = false;
                }

                if let Some(tot) = input.total {
                    counter.total = Some(tot);
                }

                if let Some(cur) = input.current {
                    counter.current = cur;
                } else {
                    let step_by = input.step.unwrap_or(1);
                    counter.current = counter.current.saturating_add(step_by);
                }
            }
            None => {
                let total = input.total;
                let current = if let Some(cur) = input.current {
                    cur
                } else if input.step == Some(0) {
                    0
                } else {
                    1
                };
                self.counters.insert(
                    name.clone(),
                    TaskCounter {
                        current,
                        total,
                        done: false,
                    },
                );
            }
        }

        let counter = self.counters.get_mut(&name).expect("counter exists");
        let (remaining, done) = match counter.total {
            Some(tot) => {
                if counter.current >= tot {
                    (Some(0), true)
                } else {
                    (Some(tot - counter.current), false)
                }
            }
            None => (None, false),
        };
        counter.done = done;

        let log_line = match counter.total {
            Some(tot) if done => format!("[Counter:{name}] {}/{} (done)", counter.current, tot),
            Some(tot) => format!(
                "[Counter:{name}] {}/{} ({} remaining)",
                counter.current,
                tot,
                remaining.unwrap_or(0)
            ),
            None => format!("[Counter:{name}] {}", counter.current),
        };
        self.sink.on_counter(&log_line);

        Ok(CounterOutput::Progress(CounterResponse {
            name,
            current: counter.current,
            total: counter.total,
            remaining,
            done,
        }))
    }
}
