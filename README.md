# sequentialthinking-rs

A fast, lightweight, and memory-safe Rust implementation of the Model Context Protocol (MCP) [Sequential Thinking Server](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking).

> **Project Status: Pre-release / Alpha**  
> This project is currently in alpha. In my own testing in local workflows it has proven stable, performant, and reliable. Community feedback and contributions are welcome.

---

## Overview

`sequentialthinking-rs` provides tools that assist LLM agents across multi-turn reasoning and execution tasks:

1. **`sequentialthinking`** (and alias **`sequentialthinking-rs`**): Enables LLMs to break down complex tasks through dynamic, reflective, and non-linear reasoning. It allows models to:
   - Adjust estimated reasoning steps (`totalThoughts`) dynamically as complexity unfolds.
   - Reconsider, revise, and refine previous deductions with explicit revision markers.
   - Branch off from earlier thoughts to explore alternative hypotheses in parallel.
   - Maintain structured reasoning context across multi-step analytical sessions.

2. **`counter`**: A lightweight, token-efficient task counter for tracking progress across iterative, multi-turn workflows without reasoning structure or prompt overhead:
   - **Initialize**: `counter({ "total": 10 })` sets `current = 1, remaining = 9, done = false`.
   - **Step**: `counter({})` auto-increments `current`.
   - **Query**: `counter({ "step": 0 })` reads current status without incrementing.
   - **Multi-Counter**: Optional `name` parameter supports concurrent named counters (defaults to `"default"`).
   - **Completion Guard**: Stepping after completion returns `"Your counter has finished, move on."`.

---

## Key Benefits & Trade-offs

### Benefits

- **Zero Runtime Dependencies:** Compiles to a single static binary. No Node.js runtime, `npm`, or `node_modules` required on the host system.
- **Minimal Resource Footprint:** Instant startup time and negligible memory usage compared to interpreted runtimes.
- **Strict Stdio Stream Safety:** Protocol communications strictly own `stdout`, while structured telemetry and thought visualization are routed safely through `stderr` via `tracing`.
- **Self-Healing LLM Error Engineering:** Validation errors are formatted with structured context (field, constraint, suggestion), allowing models to self-correct invalid tool invocations zero-shot.
- **Idempotency & Re-entrancy:** Retried or updated thoughts at an existing thought number update in-place without corrupting the sequence history or leaving dangling branches.

### Trade-offs

- **Installation Model:** Requires building from source with `cargo` or downloading a platform-specific binary, whereas the TypeScript reference can be invoked on-demand via `npx`.
- **New Features:** New features from the original `sequentialthinking` server will need to be ported in manually.

---

## Installation & Setup

### 1. Build from Source

Ensure you have a recent Rust toolchain installed (edition 2024 / Rust 1.85+ recommended):

```bash
git clone https://github.com/chrstfer/sequentialthinking-rs.git
cd sequentialthinking-rs
cargo build --release
```

The compiled binary will be located at:
```
target/release/sequentialthinking-rs
```

### 2. Configure MCP Clients

#### Claude Desktop / Claude Code (`.mcp.json` or `claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "sequentialthinking": {
      "command": "/path/to/sequentialthinking-rs",
      "args": [],
      "env": {
        "DISABLE_THOUGHT_LOGGING": "false"
      }
    }
  }
}
```

#### Antigravity / Agentic Configuration (`.agents/mcp_config.json`)

```json
{
  "mcpServers": {
    "sequentialthinking": {
      "command": "/path/to/sequentialthinking-rs"
    }
  }
}
```

---

## Configuration & Environment Variables

| Variable | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `DISABLE_THOUGHT_LOGGING` | `bool` | `false` | When set to `true` or `1`, suppresses thought log rendering to `stderr`. |
| `RUST_LOG` | `string` | `info` | Controls log filter levels for `tracing_subscriber` (e.g., `debug`, `trace`). |

---

## Testing

The project includes unit tests for schema generation and protocol integration tests:

```bash
cargo test
```

---

## Departures from the Reference Implementation

While maintaining protocol-level compatibility with the reference TypeScript server, `sequentialthinking-rs` introduces several architectural and behavioral enhancements:

| Feature / Behavior | Reference TypeScript Server | `sequentialthinking-rs` |
| :--- | :--- | :--- |
| **Duplicate Thought Numbers** | Appends duplicate entries onto history array | Performs an in-place update (upsert) and garbage-collects orphaned branch entries |
| **Response Metadata** | Basic history length and branch list | Includes `totalThoughtsAdjusted` and `replacedExisting` flags |
| **Input Key Case Tolerance** | Requires exact `camelCase` keys | Accepts standard `camelCase` with fallback `snake_case` aliases |
| **Nullable JSON Schema** | Uses TypeScript/Zod schema conventions | Generates portable `anyOf: [T, null]` schemas to ensure broad client compatibility |
| **Revision / Branch Validation** | Flexible / permissive runtime checks | Enforces relational constraints (e.g., revisions and branch origins must reference existing history) |
| **Logging Output** | Formatted ASCII boxes to `console.error` | Traditional, width-independent log output to `stderr` |
| **Iterated Task Counting** | Not supported | Dedicated lightweight `counter` tool |

---

## Community Port Notice

This project is an **independent, community-maintained port** and is **not an official release** from Anthropic or the Model Context Protocol core team. 

- **Issue Tracking & Contributions:** Please direct all bug reports, feature requests, and questions to the project repository:  
  **[github.com/chrstfer/sequentialthinking-rs/issues](https://github.com/chrstfer/sequentialthinking-rs/issues)**
- Please do not file issues regarding this Rust port in the upstream TypeScript repository.

---

## Attribution & Acknowledgments

This project is a direct port and evolution of the **Sequential Thinking MCP Server** originally developed by the **Model Context Protocol contributors** and **Anthropic PBC**:

- **Upstream Repository:** [modelcontextprotocol/servers](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking)
- **Reference Specification:** [Model Context Protocol](https://modelcontextprotocol.io/)

We are grateful to the original authors and contributors for designing the sequential thinking pattern and MCP architecture.

---

## License

This project is licensed under the **Apache License, Version 2.0** ([Apache-2.0](http://www.apache.org/licenses/LICENSE-2.0)). See the `LICENSE` file for full details.
