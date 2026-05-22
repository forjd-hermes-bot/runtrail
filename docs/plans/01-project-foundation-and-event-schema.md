# Plan 01: Project Foundation and Event Schema Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Build the Rust crate foundation, event model, validation, JSONL IO, and basic `cel log` command.

**Architecture:** Keep the event envelope in `src/event.rs`, file append/read behavior in `src/log_io.rs`, and CLI parsing in `src/cli.rs`. The CLI writes one compact JSON object per line and prints the appended event.

**Tech Stack:** Rust, clap, serde, serde_json, time, ulid, anyhow, thiserror, tempfile/assert_cmd for tests.

---

### Task 1: Configure crate metadata and dependencies

**Objective:** Set up the package as a `cel` binary with the dependencies needed for the MVP.

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`

**Steps:**
1. Update package metadata: name `compact-event-log`, binary name `cel`, license `MIT`, description.
2. Add runtime dependencies: `anyhow`, `clap` with derive, `serde`, `serde_json`, `thiserror`, `time` with formatting/parsing/macros, `ulid` with serde, and `hex` or equivalent if needed.
3. Add dev dependencies: `assert_cmd`, `predicates`, `tempfile`.
4. Run `cargo check`.
5. Commit: `chore: configure rust project foundation`.

### Task 2: Implement event model and validation tests

**Objective:** Define schema v1 and validate required fields and optional trace IDs.

**Files:**
- Create: `src/event.rs`
- Modify: `src/main.rs`

**Steps:**
1. Write failing unit tests for valid minimal event, missing/empty event name, invalid level, invalid trace/span lengths, and invalid sequence.
2. Run targeted tests and confirm they fail.
3. Implement `Event`, `Level`, `NewEvent`, `Event::new`, `Event::validate`, and timestamp/ULID generation.
4. Use `serde_json::Value` for `attrs` and `body`.
5. Run tests and `cargo check`.
6. Commit: `feat: add event schema validation`.

### Task 3: Implement JSONL read/write

**Objective:** Append events and read/validate logs as newline-delimited JSON.

**Files:**
- Create: `src/log_io.rs`
- Modify: `src/main.rs`

**Steps:**
1. Write failing tests using temp files for append creates parents, sequence increments, invalid JSON reports line number, and empty lines are ignored or handled consistently.
2. Run targeted tests and confirm they fail.
3. Implement `append_event`, `read_events`, `validate_file`, and `next_seq`.
4. Ensure writes are compact JSON plus trailing newline.
5. Run tests.
6. Commit: `feat: add jsonl log storage`.

### Task 4: Implement `cel log`

**Objective:** Expose event appending through a CLI command.

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`
- Create/modify: `tests/cli.rs`

**Steps:**
1. Write failing CLI integration tests for `cel log --event agent.note --message hello`, `--attr key=value`, `--body '{"x":1}'`, and parent directory creation.
2. Run tests and confirm they fail.
3. Implement `clap` parser and `Log` subcommand.
4. Parse attr values as JSON if possible, otherwise strings.
5. Print appended event JSON to stdout.
6. Run full tests.
7. Commit: `feat: add log command`.

### Audit checklist

- [ ] Schema matches `docs/mvp-spec.md` required fields.
- [ ] Every production behavior has tests that failed first.
- [ ] JSONL records are single-line compact JSON.
- [ ] Parent directories are created.
- [ ] Commits are conventional and one per task.
