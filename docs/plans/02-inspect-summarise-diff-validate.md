# Plan 02: Inspect, Summarise, Diff, and Validate Commands Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Add the read-side CLI commands required by the MVP: `tail`, `summarise`, `diff`, and `validate`.

**Architecture:** Keep summary aggregation in `src/summary.rs`, diff calculation in `src/diff.rs`, validation in `src/log_io.rs`, and command wiring in `src/cli.rs`.

**Tech Stack:** Rust, serde_json, clap, anyhow, assert_cmd, tempfile.

---

### Task 1: Implement `cel tail`

**Objective:** Show recent events as either human text or raw JSONL.

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/log_io.rs`
- Modify: `tests/cli.rs`

**Steps:**
1. Write failing tests for default 20 lines, `--lines 2`, and `--json` output.
2. Run tests and confirm failure.
3. Implement tail command by reading events and selecting the last N.
4. Human output should include `seq`, `ts`, `level`, and `event`.
5. Run tests.
6. Commit: `feat: add tail command`.

### Task 2: Implement `cel summarise`

**Objective:** Produce an agent-ready Markdown summary of a log.

**Files:**
- Create: `src/summary.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `tests/cli.rs`

**Steps:**
1. Write failing tests for total events, first/last timestamps, counts by event, counts by level, warnings/errors, and recent events.
2. Run tests and confirm failure.
3. Implement summary aggregation and Markdown rendering.
4. Include short previews from `body.message`, `body.error`, or compact body JSON.
5. Run tests.
6. Commit: `feat: add summarise command`.

### Task 3: Implement `cel diff`

**Objective:** Compare two logs and report count deltas and added/removed events.

**Files:**
- Create: `src/diff.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `tests/cli.rs`

**Steps:**
1. Write failing tests for added event IDs, removed event IDs, event count deltas, and new warn/error events.
2. Run tests and confirm failure.
3. Implement diff aggregation by event ID and event name.
4. Render Markdown output.
5. Run tests.
6. Commit: `feat: add diff command`.

### Task 4: Implement `cel validate`

**Objective:** Validate JSONL logs and return non-zero on invalid files.

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/log_io.rs`
- Modify: `tests/cli.rs`

**Steps:**
1. Write failing tests for valid log success, invalid JSON failure, missing required fields failure, invalid timestamp failure, and invalid trace ID failure.
2. Run tests and confirm failure.
3. Implement validate command using `validate_file`.
4. Print a clear line-numbered report.
5. Run tests.
6. Commit: `feat: add validate command`.

### Audit checklist

- [ ] Commands match names and behavior in `docs/mvp-spec.md`.
- [ ] Markdown summaries are stable enough for agents.
- [ ] Invalid logs return non-zero.
- [ ] Read-side commands do not mutate logs.
- [ ] Commits are conventional and one per task.
