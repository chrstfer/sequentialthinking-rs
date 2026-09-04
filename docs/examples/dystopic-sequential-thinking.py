#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""
sequential_thinking_server.py — Authoritative Stdio MCP Server for Sequential Thinking.

Optimized for Antigravity CLI 2.0 (AGY v1.1.23):
- Clean visual card headers with Unicode badges (`🧠 Thought 1/3 • [VERIFIED] (95% Conf)`)
- Formatted markdown blockquotes for thought content
- Hierarchical ASCII branch trees with minimal clutter
- Real-time MCP logging notifications (`notifications/message`)
- Native Mermaid (`graph TD`) and Graphviz DOT diagram exports in `summarize_thinking`

Portable and FLOSS standalone: uses only Python 3.9+ standard library with zero hardcoded personal paths.
"""

import sys
import os
import json
import datetime
import logging
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s", stream=sys.stderr)
logger = logging.getLogger("SequentialThinkingMCP")

SERVER_NAME = "sequential-thinking"
SERVER_VERSION = "2.2.0"
PROTOCOL_VERSION = "2024-11-05"

VALID_OUTPUT_MODES = {"markdown", "tree", "json", "dual", "compact", "verbose", "stream", "agy"}
VALID_DIAGRAM_FORMATS = {"mermaid", "dot", "all", "none"}


def resolve_default_log_dir() -> Path:
    """Resolves a cross-platform, non-hardcoded directory for sequential thinking telemetry logs."""
    env_dir = os.environ.get("SEQUENTIAL_THINKING_LOG_DIR")
    if env_dir:
        return Path(env_dir)
    try:
        home_dir = Path.home() / ".sequential_thinking" / "logs"
        home_dir.mkdir(parents=True, exist_ok=True)
        return home_dir
    except Exception:
        fallback = Path(os.environ.get("TEMP", os.environ.get("TMPDIR", "/tmp"))) / "sequential_thinking_logs"
        try:
            fallback.mkdir(parents=True, exist_ok=True)
            return fallback
        except Exception:
            return Path(".")


class ThoughtManager:
    """Manages sequential thinking state, multi-session graphs, hypothesis confidence, revisions, and output rendering."""

    def __init__(self):
        # session_id -> { "thoughts": [], "branches": {}, "created_at": iso, "last_updated": iso }
        self.sessions: Dict[str, Dict[str, Any]] = {}
        self.default_output_mode: str = os.environ.get("SEQUENTIAL_THINKING_OUTPUT_MODE", "markdown").lower().strip()
        if self.default_output_mode not in VALID_OUTPUT_MODES:
            self.default_output_mode = "markdown"
        self.log_dir: Path = resolve_default_log_dir()
        self.log_enabled: bool = os.environ.get("SEQUENTIAL_THINKING_LOG_ENABLED", "1").lower() in ("1", "true", "yes")
        self.stream_stderr: bool = os.environ.get("SEQUENTIAL_THINKING_STREAM_STDERR", "0").lower() in ("1", "true", "yes")
        self.show_history_tree: bool = True
        self.default_diagram_format: str = os.environ.get("SEQUENTIAL_THINKING_DIAGRAM_FORMAT", "mermaid").lower().strip()
        self.send_mcp_notifications: bool = os.environ.get("SEQUENTIAL_THINKING_NOTIFICATIONS", "1").lower() in ("1", "true", "yes")

    def configure(
        self,
        mode: Optional[str] = None,
        log_enabled: Optional[bool] = None,
        log_dir: Optional[str] = None,
        stream_stderr: Optional[bool] = None,
        show_history_tree: Optional[bool] = None,
        diagram_format: Optional[str] = None,
        send_mcp_notifications: Optional[bool] = None,
    ) -> Dict[str, Any]:
        """Dynamically updates global output modes and logging settings."""
        if mode:
            m = mode.lower().strip()
            if m in VALID_OUTPUT_MODES:
                self.default_output_mode = m
            else:
                raise ValueError(f"Invalid mode '{mode}'. Choose from: {sorted(VALID_OUTPUT_MODES)}")

        if log_enabled is not None:
            self.log_enabled = bool(log_enabled)

        if log_dir:
            p = Path(log_dir)
            p.mkdir(parents=True, exist_ok=True)
            self.log_dir = p

        if stream_stderr is not None:
            self.stream_stderr = bool(stream_stderr)

        if show_history_tree is not None:
            self.show_history_tree = bool(show_history_tree)

        if diagram_format:
            df = diagram_format.lower().strip()
            if df in VALID_DIAGRAM_FORMATS:
                self.default_diagram_format = df
            else:
                raise ValueError(f"Invalid diagram format '{diagram_format}'. Choose from: {sorted(VALID_DIAGRAM_FORMATS)}")

        if send_mcp_notifications is not None:
            self.send_mcp_notifications = bool(send_mcp_notifications)

        return {
            "status": "CONFIGURED",
            "default_output_mode": self.default_output_mode,
            "log_enabled": self.log_enabled,
            "log_dir": str(self.log_dir),
            "stream_stderr": self.stream_stderr,
            "show_history_tree": self.show_history_tree,
            "diagram_format": self.default_diagram_format,
            "send_mcp_notifications": self.send_mcp_notifications,
        }

    def _get_session(self, session_id: Optional[str] = None) -> Dict[str, Any]:
        sid = (session_id or "default").strip()
        now_iso = datetime.datetime.now(datetime.timezone.utc).isoformat()
        if sid not in self.sessions:
            self.sessions[sid] = {
                "thoughts": [],
                "branches": {},
                "created_at": now_iso,
                "last_updated": now_iso,
            }
        return self.sessions[sid]

    def _write_log_entry(self, thought_data: Dict[str, Any]) -> None:
        if not self.log_enabled:
            return
        try:
            self.log_dir.mkdir(parents=True, exist_ok=True)
            log_file = self.log_dir / "sequential_thinking.jsonl"
            with open(log_file, "a", encoding="utf-8") as f:
                f.write(json.dumps(thought_data, ensure_ascii=False) + "\n")
        except Exception as e:
            logger.debug(f"Failed to write thought log entry: {e}")

    def _render_tree_ascii(self, session: Dict[str, Any], current_thought: Dict[str, Any]) -> str:
        """Renders a clean visual ASCII/Unicode graph of all thoughts, branches, and revisions in the session."""
        main_thoughts = session["thoughts"]
        branches = session["branches"]
        
        all_main = list(main_thoughts)
        lines = []
        
        for idx, t in enumerate(all_main):
            is_last_main = (idx == len(all_main) - 1) and not branches
            prefix = "└── " if is_last_main else "├── "
            t_num = t["thought_number"]
            total = t["total_thoughts"]
            status = f"[{t.get('hypothesis_status', 'EXPLORING')}]"
            conf = f" ({int(t['confidence']*100)}%)" if t.get("confidence") is not None else ""
            rev = f" ↳ Revises #{t['revises_thought']}" if t.get("is_revision") else ""
            
            snippet = t["thought"].replace("\n", " ").strip()
            if len(snippet) > 55:
                snippet = snippet[:52] + "..."
                
            is_active = " ⭐" if t["thought_number"] == current_thought.get("thought_number") and not current_thought.get("branch_id") else ""
            lines.append(f"{prefix}[{t_num}/{total}] {status}{conf}{rev} {snippet}{is_active}")
            
            # Check for branches originating from this thought
            for bid, bthoughts in branches.items():
                if bthoughts and bthoughts[0].get("branch_from_thought") == t_num:
                    branch_pipe = "    " if is_last_main else "│   "
                    lines.append(f"{branch_pipe}├── 🌿 Branch `{bid}` ({len(bthoughts)} steps):")
                    for b_idx, bt in enumerate(bthoughts):
                        b_is_last = (b_idx == len(bthoughts) - 1)
                        b_pfx = "└── " if b_is_last else "├── "
                        b_conf = f" ({int(bt['confidence']*100)}%)" if bt.get("confidence") is not None else ""
                        b_snip = bt["thought"].replace("\n", " ").strip()
                        if len(b_snip) > 48:
                            b_snip = b_snip[:45] + "..."
                        b_active = " ⭐" if bt["thought_number"] == current_thought.get("thought_number") and current_thought.get("branch_id") == bid else ""
                        lines.append(f"{branch_pipe}│   {b_pfx}Step {bt['thought_number']}: {b_snip}{b_conf}{b_active}")

        # Isolated branches without branch_from_thought match
        matched_bids = {
            bid for bid, bthoughts in branches.items() 
            if bthoughts and any(t["thought_number"] == bthoughts[0].get("branch_from_thought") for t in all_main)
        }
        for bid, bthoughts in branches.items():
            if bid not in matched_bids and bthoughts:
                lines.append(f"├── 🌿 Independent Branch `{bid}` ({len(bthoughts)} steps):")
                for b_idx, bt in enumerate(bthoughts):
                    b_pfx = "└── " if b_idx == len(bthoughts) - 1 else "├── "
                    b_snip = bt["thought"].replace("\n", " ").strip()
                    if len(b_snip) > 48:
                        b_snip = b_snip[:45] + "..."
                    lines.append(f"│   {b_pfx}Step {bt['thought_number']}: {b_snip}")

        return "\n".join(lines)

    def _render_mermaid(self, session: Dict[str, Any], winning_branch_id: Optional[str] = None) -> str:
        """Renders a Mermaid flowchart representing the thought graph, confidence, and revisions."""
        main_thoughts = session["thoughts"]
        branches = session["branches"]

        lines = ["```mermaid", "graph TD"]
        lines.append("    classDef verified fill:#2e7d32,stroke:#1b5e20,color:#ffffff,stroke-width:2px;")
        lines.append("    classDef exploring fill:#1565c0,stroke:#0d47a1,color:#ffffff,stroke-width:2px;")
        lines.append("    classDef refuted fill:#c62828,stroke:#8e0000,color:#ffffff,stroke-width:2px;")
        lines.append("    classDef branch fill:#6a1b9a,stroke:#4a148c,color:#ffffff,stroke-width:2px;")
        lines.append("    classDef winning fill:#f57f17,stroke:#bc5100,color:#ffffff,stroke-width:3px;")

        # 1. Main thoughts
        prev_node = None
        for t in main_thoughts:
            t_num = t["thought_number"]
            node_id = f"T{t_num}"
            raw_txt = t["thought"].replace('"', "'").replace("\n", " ").strip()
            if len(raw_txt) > 40:
                raw_txt = raw_txt[:37] + "..."
            status = t.get("hypothesis_status", "EXPLORING").upper()
            conf_str = f"<br/><i>Conf: {int(t['confidence']*100)}%</i>" if t.get("confidence") is not None else ""
            label = f"\"[{t_num}/{t['total_thoughts']}] {raw_txt}<br/><b>[{status}]</b>{conf_str}\""

            lines.append(f"    {node_id}[{label}]")
            
            # Apply style class
            cls = "verified" if status == "VERIFIED" else ("refuted" if status == "REFUTED" else "exploring")
            lines.append(f"    class {node_id} {cls};")

            if prev_node:
                lines.append(f"    {prev_node} --> {node_id}")
            prev_node = node_id

            # Revisions
            if t.get("is_revision") and t.get("revises_thought"):
                target_node = f"T{t['revises_thought']}"
                lines.append(f"    {node_id} -. revises .-> {target_node}")

        # 2. Branches
        for bid, bthoughts in branches.items():
            b_prev = None
            is_winner = (winning_branch_id and bid == winning_branch_id)
            for bt in bthoughts:
                bt_num = bt["thought_number"]
                b_node_id = f"B_{bid}_{bt_num}"
                raw_b = bt["thought"].replace('"', "'").replace("\n", " ").strip()
                if len(raw_b) > 35:
                    raw_b = raw_b[:32] + "..."
                conf_b = f"<br/><i>Conf: {int(bt['confidence']*100)}%</i>" if bt.get("confidence") is not None else ""
                b_label = f"\"[Branch {bid}] Step {bt_num}<br/>{raw_b}{conf_b}\""

                lines.append(f"    {b_node_id}[{b_label}]")
                lines.append(f"    class {b_node_id} {'winning' if is_winner else 'branch'};")

                if b_prev:
                    lines.append(f"    {b_prev} --> {b_node_id}")
                else:
                    origin_num = bt.get("branch_from_thought")
                    if origin_num:
                        lines.append(f"    T{origin_num} -. fork `{bid}` .-> {b_node_id}")
                b_prev = b_node_id

        lines.append("```")
        return "\n".join(lines)

    def _render_dot(self, session: Dict[str, Any], winning_branch_id: Optional[str] = None) -> str:
        """Renders Graphviz DOT syntax representing the thought graph."""
        main_thoughts = session["thoughts"]
        branches = session["branches"]

        lines = [
            "```dot",
            "digraph SequentialThinking {",
            "    rankdir=TB;",
            "    node [shape=box, style=\"rounded,filled\", fontname=\"Helvetica\", fontsize=10];",
            "    edge [fontname=\"Helvetica\", fontsize=9];"
        ]

        prev_node = None
        for t in main_thoughts:
            t_num = t["thought_number"]
            node_id = f"T{t_num}"
            raw_txt = t["thought"].replace('"', "'").replace("\n", " ").strip()
            if len(raw_txt) > 35:
                raw_txt = raw_txt[:32] + "..."
            status = t.get("hypothesis_status", "EXPLORING").upper()
            color = "#c8e6c9" if status == "VERIFIED" else ("#ffcdd2" if status == "REFUTED" else "#bbdefb")
            label = f"[{t_num}/{t['total_thoughts']}] {raw_txt}\\nStatus: {status}"
            lines.append(f"    {node_id} [label=\"{label}\", fillcolor=\"{color}\"];")

            if prev_node:
                lines.append(f"    {prev_node} -> {node_id};")
            prev_node = node_id

            if t.get("is_revision") and t.get("revises_thought"):
                target_node = f"T{t['revises_thought']}"
                lines.append(f"    {node_id} -> {target_node} [style=dashed, color=red, label=\"revises\"];")

        for bid, bthoughts in branches.items():
            b_prev = None
            is_winner = (winning_branch_id and bid == winning_branch_id)
            color = "#ffe082" if is_winner else "#e1bee7"
            for bt in bthoughts:
                bt_num = bt["thought_number"]
                b_node_id = f"B_{bid}_{bt_num}"
                raw_b = bt["thought"].replace('"', "'").replace("\n", " ").strip()
                if len(raw_b) > 30:
                    raw_b = raw_b[:27] + "..."
                b_label = f"[{bid}] Step {bt_num}\\n{raw_b}"
                lines.append(f"    {b_node_id} [label=\"{b_label}\", fillcolor=\"{color}\"];")

                if b_prev:
                    lines.append(f"    {b_prev} -> {b_node_id};")
                else:
                    origin_num = bt.get("branch_from_thought")
                    if origin_num:
                        lines.append(f"    T{origin_num} -> {b_node_id} [style=dotted, color=purple, label=\"branch {bid}\"];")
                b_prev = b_node_id

        lines.append("}")
        lines.append("```")
        return "\n".join(lines)

    def process_thought(
        self,
        thought: str,
        next_thought_needed: bool,
        thought_number: int,
        total_thoughts: int,
        is_revision: Optional[bool] = False,
        revises_thought: Optional[int] = None,
        branch_from_thought: Optional[int] = None,
        branch_id: Optional[str] = None,
        needs_more_thoughts: Optional[bool] = False,
        session_id: Optional[str] = "default",
        confidence: Optional[float] = None,
        hypothesis_status: Optional[str] = None,
        output_mode: Optional[str] = None,
    ) -> Dict[str, Any]:
        sess = self._get_session(session_id)
        now_iso = datetime.datetime.now(datetime.timezone.utc).isoformat()
        sess["last_updated"] = now_iso

        thought_data = {
            "thought_number": thought_number,
            "total_thoughts": total_thoughts,
            "thought": thought,
            "next_thought_needed": next_thought_needed,
            "is_revision": is_revision,
            "revises_thought": revises_thought,
            "branch_from_thought": branch_from_thought,
            "branch_id": branch_id,
            "needs_more_thoughts": needs_more_thoughts,
            "session_id": session_id or "default",
            "confidence": confidence,
            "hypothesis_status": hypothesis_status or "EXPLORING",
            "timestamp": now_iso,
        }

        if branch_id:
            if branch_id not in sess["branches"]:
                sess["branches"][branch_id] = []
            sess["branches"][branch_id].append(thought_data)
        else:
            sess["thoughts"].append(thought_data)

        # Write telemetry log
        self._write_log_entry(thought_data)

        # Determine effective output mode
        eff_mode = (output_mode or self.default_output_mode).lower().strip()
        if eff_mode not in VALID_OUTPUT_MODES:
            eff_mode = "markdown"

        # Optional stderr streaming
        if self.stream_stderr or eff_mode == "stream":
            sys.stderr.write(f"[SequentialThinking] Session={session_id} Step={thought_number}/{total_thoughts} Status={hypothesis_status or 'EXPLORING'} Conf={confidence}\n")
            sys.stderr.write(f"  Thought: {thought}\n")
            sys.stderr.flush()

        # Build output representation according to selected mode
        status_text = self._format_output(eff_mode, sess, thought_data)

        return {
            "thought_number": thought_number,
            "total_thoughts": total_thoughts,
            "next_thought_needed": next_thought_needed,
            "branches": list(sess["branches"].keys()),
            "output_mode": eff_mode,
            "status_text": status_text,
            "raw_thought": thought_data,
        }

    def _format_output(self, mode: str, session: Dict[str, Any], thought_data: Dict[str, Any]) -> str:
        thought_number = thought_data["thought_number"]
        total_thoughts = thought_data["total_thoughts"]
        branch_id = thought_data["branch_id"]
        session_id = thought_data["session_id"]
        is_revision = thought_data["is_revision"]
        revises_thought = thought_data["revises_thought"]
        branch_from_thought = thought_data["branch_from_thought"]
        hypothesis_status = thought_data["hypothesis_status"]
        confidence = thought_data["confidence"]
        thought = thought_data["thought"]
        next_thought_needed = thought_data["next_thought_needed"]

        if mode == "json":
            return json.dumps({
                "current_thought": thought_data,
                "session_state": {
                    "session_id": session_id,
                    "total_main_thoughts": len(session["thoughts"]),
                    "active_branches": list(session["branches"].keys()),
                    "last_updated": session["last_updated"]
                }
            }, indent=2)

        elif mode == "compact":
            branch_str = f" [Branch: {branch_id}]" if branch_id else ""
            status_str = f" [{hypothesis_status}]" if hypothesis_status and hypothesis_status != "EXPLORING" else ""
            conf_str = f" (Conf: {int(confidence*100)}%)" if confidence is not None else ""
            return f"[Thought {thought_number}/{total_thoughts}]{branch_str}{status_str}{conf_str}: {thought}"

        elif mode == "verbose":
            lines = [
                "=================================================================",
                f" SEQUENTIAL THINKING — STEP {thought_number}/{total_thoughts}",
                "=================================================================",
                f"Session ID:          {session_id}",
                f"Timestamp:           {thought_data['timestamp']}",
                f"Next Step Needed:    {'Yes' if next_thought_needed else 'No (Final Thought)'}",
                f"Hypothesis Status:   {hypothesis_status}",
                f"Confidence Level:    {int(confidence * 100) if confidence is not None else 'N/A'}%",
            ]
            if is_revision:
                lines.append(f"Revision Target:     Thought #{revises_thought}")
            if branch_id:
                lines.append(f"Active Branch:       {branch_id} (Branched from #{branch_from_thought})")
            lines.append("-----------------------------------------------------------------")
            lines.append(f"Thought Content:\n{thought}")
            lines.append("-----------------------------------------------------------------")
            lines.append(self._render_tree_ascii(session, thought_data))
            lines.append("=================================================================")
            return "\n".join(lines)

        else:  # markdown / tree / dual / stream / agy (Optimized for AGY CLI 2.0 / v1.1.23)
            # 1. Clean visual header line
            header_items = [f"🧠 **Thought {thought_number}/{total_thoughts}**"]
            if branch_id:
                header_items.append(f"🌿 `{branch_id}`")
            if hypothesis_status and hypothesis_status.upper() != "EXPLORING":
                header_items.append(f"`[{hypothesis_status.upper()}]`")
            if confidence is not None:
                pct = int(confidence * 100) if 0.0 <= confidence <= 1.0 else int(confidence)
                header_items.append(f"*({pct}% Conf)*")
            
            lines = [" • ".join(header_items)]

            # 2. Revision or Branching origin sub-indicator
            sub_meta = []
            if is_revision and revises_thought:
                sub_meta.append(f"↳ *Revises Thought #{revises_thought}*")
            if branch_from_thought:
                sub_meta.append(f"↳ *Forked from Thought #{branch_from_thought}*")
            if sub_meta:
                lines.append(" ".join(sub_meta))

            # 3. Clean thought body in quote block
            lines.append("")
            lines.append(f"> {thought.strip()}")
            lines.append("")

            # 4. Status footer
            next_label = "Next: Continuous" if next_thought_needed else "✅ Final Step"
            footer_items = [f"*{next_label}*"]
            if session_id and session_id != "default":
                footer_items.append(f"*Session: `{session_id}`*")
            lines.append(" • ".join(footer_items))

            # 5. Clean ASCII Tree if multi-thought
            if self.show_history_tree and (len(session["thoughts"]) + len(session["branches"]) > 1):
                lines.append("")
                lines.append("```text")
                lines.append(self._render_tree_ascii(session, thought_data))
                lines.append("```")

            return "\n".join(lines)

    def summarize_thinking(
        self,
        session_id: Optional[str] = "default",
        winning_branch_id: Optional[str] = None,
        output_mode: Optional[str] = None,
        diagram_format: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Synthesizes the active decision path into a markdown or json decision trace with Mermaid/Graphviz diagrams."""
        sess = self._get_session(session_id)
        main_thoughts = sess["thoughts"]
        branches = sess["branches"]
        eff_mode = (output_mode or self.default_output_mode).lower().strip()
        eff_diag = (diagram_format or self.default_diagram_format).lower().strip()
        if eff_diag not in VALID_DIAGRAM_FORMATS:
            eff_diag = "mermaid"

        if not main_thoughts and not branches:
            return {
                "session_id": session_id,
                "summary_markdown": f"No thoughts recorded for session '{session_id}'.",
                "total_thoughts": 0,
            }

        if eff_mode == "json":
            return {
                "session_id": session_id,
                "winning_branch_id": winning_branch_id,
                "main_thoughts": main_thoughts,
                "branches": branches,
                "total_thoughts": len(main_thoughts) + sum(len(b) for b in branches.values()),
                "summary_markdown": json.dumps({
                    "session_id": session_id,
                    "main_thoughts": main_thoughts,
                    "branches": branches,
                }, indent=2)
            }

        lines = [
            f"### 🧠 Sequential Thinking Summary — `{session_id}`",
            f"**Recorded Steps:** {len(main_thoughts)} main thoughts | **Active Branches:** {len(branches)}",
            "",
            "#### Decision Path & Hypotheses:",
        ]

        # 1. Main sequence
        for t in main_thoughts:
            prefix = f"**Step {t['thought_number']}/{t['total_thoughts']}:**"
            status_tag = f" `[{t['hypothesis_status']}]`" if t.get("hypothesis_status") and t["hypothesis_status"] != "EXPLORING" else ""
            conf_tag = f" *(Confidence: {int(t['confidence']*100)}%)*" if t.get("confidence") is not None else ""
            rev_tag = f" *(Revises Step #{t['revises_thought']})*" if t.get("is_revision") else ""
            lines.append(f"- {prefix}{status_tag}{conf_tag}{rev_tag} {t['thought']}")

        # 2. Selected or winning branch
        if winning_branch_id and winning_branch_id in branches:
            lines.append("")
            lines.append(f"#### Selected Branch `{winning_branch_id}`:")
            for b in branches[winning_branch_id]:
                lines.append(f"  - **Branch Step {b['thought_number']}:** {b['thought']}")
        elif branches:
            lines.append("")
            lines.append("#### Explored Alternative Branches:")
            for bid, bthoughts in branches.items():
                lines.append(f"- **Branch `{bid}`** ({len(bthoughts)} steps):")
                for b in bthoughts:
                    lines.append(f"  - Step {b['thought_number']}: {b['thought']}")

        # 3. Overall Thought Tree
        if self.show_history_tree and (len(main_thoughts) + len(branches) > 1):
            lines.append("")
            lines.append("#### Thought Graph Tree:")
            lines.append("```text")
            dummy_curr = main_thoughts[-1] if main_thoughts else {}
            lines.append(self._render_tree_ascii(sess, dummy_curr))
            lines.append("```")

        # 4. Visual Diagrams (Mermaid / Graphviz)
        if eff_diag in ("mermaid", "all") and (len(main_thoughts) + len(branches) > 1):
            lines.append("")
            lines.append("#### Flowchart Graph (Mermaid):")
            lines.append(self._render_mermaid(sess, winning_branch_id))

        if eff_diag in ("dot", "all") and (len(main_thoughts) + len(branches) > 1):
            lines.append("")
            lines.append("#### Graphviz DOT Definition:")
            lines.append(self._render_dot(sess, winning_branch_id))

        return {
            "session_id": session_id,
            "summary_markdown": "\n".join(lines),
            "total_thoughts": len(main_thoughts) + sum(len(b) for b in branches.values()),
        }

    def reset_thinking(self, session_id: Optional[str] = None, all_sessions: bool = False) -> Dict[str, Any]:
        """Clears thought history."""
        if all_sessions:
            count = len(self.sessions)
            self.sessions.clear()
            return {"status": "CLEARED_ALL", "cleared_sessions": count, "message": "All thought sessions cleared."}
        else:
            sid = (session_id or "default").strip()
            if sid in self.sessions:
                del self.sessions[sid]
                return {"status": "CLEARED", "session_id": sid, "message": f"Session '{sid}' cleared."}
            return {"status": "NOT_FOUND", "session_id": sid, "message": f"Session '{sid}' did not exist."}

    def get_state(self, session_id: Optional[str] = None) -> Dict[str, Any]:
        """Inspects the live multi-session thought tree and branch state."""
        if session_id:
            sid = session_id.strip()
            sess = self.sessions.get(sid, {"thoughts": [], "branches": {}, "created_at": None, "last_updated": None})
            return {"session_id": sid, "session_data": sess}
        return {
            "total_sessions": len(self.sessions),
            "sessions": {sid: {"total_thoughts": len(s["thoughts"]), "branches": list(s["branches"].keys()), "last_updated": s["last_updated"]} for sid, s in self.sessions.items()}
        }


thought_manager = ThoughtManager()

TOOLS = [
    {
        "name": "sequentialthinking",
        "description": "A detailed tool for dynamic and reflective problem-solving through sequential thinking steps with clean AGY 2.0 formatting, visual tree graphs, and real-time streaming.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Your current thinking step content"
                },
                "nextThoughtNeeded": {
                    "type": "boolean",
                    "description": "Whether another thought step is needed"
                },
                "thoughtNumber": {
                    "type": "integer",
                    "description": "Current thought number in sequence",
                    "minimum": 1
                },
                "totalThoughts": {
                    "type": "integer",
                    "description": "Estimated total thoughts needed",
                    "minimum": 1
                },
                "isRevision": {
                    "type": "boolean",
                    "description": "Whether this thought revises a previous thought"
                },
                "revisesThought": {
                    "type": "integer",
                    "description": "Which thought number is being revised"
                },
                "branchFromThought": {
                    "type": "integer",
                    "description": "Which thought number to branch from"
                },
                "branchId": {
                    "type": "string",
                    "description": "Identifier for the current alternative branch"
                },
                "needsMoreThoughts": {
                    "type": "boolean",
                    "description": "Whether additional thinking steps beyond totalThoughts are required"
                },
                "sessionId": {
                    "type": "string",
                    "description": "Unique identifier for multi-session or multi-agent graph isolation (defaults to 'default')"
                },
                "confidence": {
                    "type": "number",
                    "description": "Confidence score in current hypothesis (0.0 to 1.0 or 0 to 100)"
                },
                "hypothesisStatus": {
                    "type": "string",
                    "description": "Status of hypothesis (e.g. EXPLORING, VERIFIED, REFUTED, STALLED, REVISED)"
                },
                "outputMode": {
                    "type": "string",
                    "description": "Output presentation mode: 'markdown', 'agy', 'json', 'dual', 'compact', 'verbose', 'stream'",
                    "enum": ["markdown", "agy", "tree", "json", "dual", "compact", "verbose", "stream"]
                }
            },
            "required": ["thought", "nextThoughtNeeded", "thoughtNumber", "totalThoughts"]
        }
    },
    {
        "name": "summarize_thinking",
        "description": "Synthesizes the active decision path and hypotheses across main thoughts and branches into a clean markdown or json report with Mermaid/Graphviz diagrams.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "sessionId": {
                    "type": "string",
                    "description": "Session identifier to summarize (defaults to 'default')"
                },
                "winningBranchId": {
                    "type": "string",
                    "description": "Optional identifier of the selected winning branch to highlight"
                },
                "outputMode": {
                    "type": "string",
                    "description": "Format of the summary: 'markdown' or 'json'",
                    "enum": ["markdown", "json"]
                },
                "diagramFormat": {
                    "type": "string",
                    "description": "Visual diagram format to generate: 'mermaid', 'dot', 'all', or 'none'",
                    "enum": ["mermaid", "dot", "all", "none"]
                }
            }
        }
    },
    {
        "name": "reset_thinking",
        "description": "Clears thought history for a specific session or resets all sessions entirely.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "sessionId": {
                    "type": "string",
                    "description": "Session identifier to reset"
                },
                "allSessions": {
                    "type": "boolean",
                    "description": "Set to true to purge all session histories globally"
                }
            }
        }
    },
    {
        "name": "configure_thinking",
        "description": "Configures global sequential thinking output representation modes, diagram formats, persistent telemetry logging, and display options.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "description": "Default output mode: 'markdown', 'agy', 'json', 'dual', 'compact', 'verbose', or 'stream'",
                    "enum": ["markdown", "agy", "tree", "json", "dual", "compact", "verbose", "stream"]
                },
                "logEnabled": {
                    "type": "boolean",
                    "description": "Enable or disable writing thoughts to JSONL telemetry log"
                },
                "logDir": {
                    "type": "string",
                    "description": "Custom directory path to save JSONL logs"
                },
                "streamStderr": {
                    "type": "boolean",
                    "description": "Stream thought steps and status directly to stderr for live console monitoring"
                },
                "showHistoryTree": {
                    "type": "boolean",
                    "description": "Render ASCII/Unicode thought graph tree in Markdown output"
                },
                "diagramFormat": {
                    "type": "string",
                    "description": "Default diagram format: 'mermaid', 'dot', 'all', or 'none'",
                    "enum": ["mermaid", "dot", "all", "none"]
                },
                "sendMcpNotifications": {
                    "type": "boolean",
                    "description": "Send live MCP protocol logging notifications on every step"
                }
            }
        }
    },
    {
        "name": "get_thinking_state",
        "description": "Inspects the live multi-session thought tree and branch state without mutating thought history.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "sessionId": {
                    "type": "string",
                    "description": "Optional session identifier to inspect in detail"
                }
            }
        }
    }
]


def handle_rpc_request(request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
    """Processes an incoming JSON-RPC 2.0 request and returns the response."""
    req_id = request.get("id")
    method = request.get("method")
    params = request.get("params", {})

    if method == "initialize":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {},
                    "logging": {}
                },
                "serverInfo": {
                    "name": SERVER_NAME,
                    "version": SERVER_VERSION
                }
            }
        }

    elif method == "notifications/initialized":
        return None

    elif method == "tools/list":
        return {
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "tools": TOOLS
            }
        }

    elif method == "tools/call":
        tool_name = params.get("name")
        args = params.get("arguments", {})

        try:
            if tool_name == "sequentialthinking":
                thought = args.get("thought", "")
                next_thought = bool(args.get("nextThoughtNeeded", False))
                thought_num = int(args.get("thoughtNumber", 1))
                total_thoughts = int(args.get("totalThoughts", 1))
                is_rev = bool(args.get("isRevision", False))
                rev_num = args.get("revisesThought")
                branch_from = args.get("branchFromThought")
                branch_id = args.get("branchId")
                needs_more = bool(args.get("needsMoreThoughts", False))
                sess_id = args.get("sessionId", "default")
                conf = args.get("confidence")
                hyp_stat = args.get("hypothesisStatus")
                out_mode = args.get("outputMode")

                res = thought_manager.process_thought(
                    thought=thought,
                    next_thought_needed=next_thought,
                    thought_number=thought_num,
                    total_thoughts=total_thoughts,
                    is_revision=is_rev,
                    revises_thought=int(rev_num) if rev_num is not None else None,
                    branch_from_thought=int(branch_from) if branch_from is not None else None,
                    branch_id=branch_id,
                    needs_more_thoughts=needs_more,
                    session_id=sess_id,
                    confidence=float(conf) if conf is not None else None,
                    hypothesis_status=hyp_stat,
                    output_mode=out_mode,
                )

                # Send MCP protocol log notification if enabled
                if thought_manager.send_mcp_notifications:
                    log_notif = {
                        "jsonrpc": "2.0",
                        "method": "notifications/message",
                        "params": {
                            "level": "info",
                            "logger": "sequential-thinking",
                            "data": {
                                "sessionId": sess_id,
                                "step": f"{thought_num}/{total_thoughts}",
                                "status": hyp_stat or "EXPLORING",
                                "confidence": conf,
                                "branchId": branch_id,
                                "thought": thought[:100] + ("..." if len(thought) > 100 else "")
                            }
                        }
                    }
                    try:
                        sys.stdout.write(json.dumps(log_notif, ensure_ascii=False) + "\n")
                        sys.stdout.flush()
                    except Exception:
                        pass

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": res["status_text"]
                            }
                        ],
                        "isError": False
                    }
                }

            elif tool_name == "summarize_thinking":
                sess_id = args.get("sessionId", "default")
                winning_branch = args.get("winningBranchId")
                out_mode = args.get("outputMode")
                diag_fmt = args.get("diagramFormat")
                summary = thought_manager.summarize_thinking(
                    session_id=sess_id,
                    winning_branch_id=winning_branch,
                    output_mode=out_mode,
                    diagram_format=diag_fmt
                )

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": summary["summary_markdown"]
                            }
                        ],
                        "isError": False
                    }
                }

            elif tool_name == "reset_thinking":
                sess_id = args.get("sessionId", "default")
                all_sess = bool(args.get("allSessions", False))
                res = thought_manager.reset_thinking(session_id=sess_id, all_sessions=all_sess)

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(res, indent=2)
                            }
                        ],
                        "isError": False
                    }
                }

            elif tool_name == "configure_thinking":
                res = thought_manager.configure(
                    mode=args.get("mode"),
                    log_enabled=args.get("logEnabled"),
                    log_dir=args.get("logDir"),
                    stream_stderr=args.get("streamStderr"),
                    show_history_tree=args.get("showHistoryTree"),
                    diagram_format=args.get("diagramFormat"),
                    send_mcp_notifications=args.get("sendMcpNotifications"),
                )

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(res, indent=2)
                            }
                        ],
                        "isError": False
                    }
                }

            elif tool_name == "get_thinking_state":
                sess_id = args.get("sessionId")
                res = thought_manager.get_state(session_id=sess_id)

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": json.dumps(res, indent=2)
                            }
                        ],
                        "isError": False
                    }
                }

            else:
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32601,
                        "message": f"Method/Tool '{tool_name}' not found"
                    }
                }

        except Exception as e:
            logger.exception("Error executing sequential thinking tool")
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": f"Execution Error: {str(e)}"
                        }
                    ],
                    "isError": True
                }
            }

    else:
        if req_id is not None:
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": f"Unhandled method: {method}"
                }
            }
        return None


def main():
    """Main stdio loop reading JSON-RPC lines or Content-Length frames."""
    while True:
        try:
            line = sys.stdin.readline()
            if not line:
                break

            if line.startswith("Content-Length:"):
                length = int(line.split(":")[1].strip())
                # Read empty line delimiter
                while True:
                    empty_line = sys.stdin.readline()
                    if empty_line in ("\r\n", "\n", ""):
                        break
                body = sys.stdin.read(length)
                if not body:
                    break
                data = json.loads(body)
            else:
                line_str = line.strip()
                if not line_str:
                    continue
                data = json.loads(line_str)

            response = handle_rpc_request(data)
            if response is not None:
                resp_str = json.dumps(response, ensure_ascii=False)
                sys.stdout.write(resp_str + "\n")
                sys.stdout.flush()

        except json.JSONDecodeError as e:
            err_resp = {
                "jsonrpc": "2.0",
                "id": None,
                "error": {
                    "code": -32700,
                    "message": f"Parse error: {str(e)}"
                }
            }
            sys.stdout.write(json.dumps(err_resp) + "\n")
            sys.stdout.flush()
        except Exception as e:
            logger.exception("Fatal exception in stdio loop")
            err_resp = {
                "jsonrpc": "2.0",
                "id": None,
                "error": {
                    "code": -32603,
                    "message": f"Internal error: {str(e)}"
                }
            }
            sys.stdout.write(json.dumps(err_resp) + "\n")
            sys.stdout.flush()


if __name__ == "__main__":
    main()