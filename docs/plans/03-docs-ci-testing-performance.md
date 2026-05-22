# Plan 03: Documentation, CI Context, Testing, and Performance Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Finish MVP polish: README, CI context helper behavior if feasible, thorough tests, performance smoke testing, and public-release readiness.

**Architecture:** Documentation stays in README/docs. Any GitHub Actions environment capture is implemented as a small command or helper that emits a normal event through existing log IO.

**Tech Stack:** Rust, Cargo, shell, GitHub CLI.

---

### Task 1: Write README and usage docs

**Objective:** Make the repo understandable for public GitHub users.

**Files:**
- Modify: `README.md`

**Steps:**
1. Document pitch, install/build, schema, commands, examples, and MVP status.
2. Include examples for command, browser, test, repo, agent note, and CI status events.
3. Include interoperability examples with `jq`.
4. Run `cargo test` to ensure docs changes did not affect code.
5. Commit: `docs: add mvp readme`.

### Task 2: Add `ci github-context` helper

**Objective:** Capture GitHub Actions environment metadata as a `ci.github.context` event.

**Files:**
- Modify: `src/cli.rs`
- Modify: `tests/cli.rs`

**Steps:**
1. Write failing CLI tests that set `GITHUB_RUN_ID`, `GITHUB_RUN_ATTEMPT`, `GITHUB_WORKFLOW`, `GITHUB_SHA`, and `GITHUB_REPOSITORY`, then run `runtrail ci github-context`.
2. Confirm failure.
3. Implement nested `ci github-context` command that reads a safe allowlist of GitHub/runner env vars and appends an event.
4. Run tests.
5. Commit: `feat: add github actions context logging`.

### Task 3: Add performance fixture and benchmark command/script

**Objective:** Create reproducible performance smoke checks for 10k+ events.

**Files:**
- Create: `scripts/perf-smoke.sh`
- Modify: `README.md`

**Steps:**
1. Write a shell script that builds release, generates a temp 10k-event log with `runtrail log`, then times `validate` and `summarise`.
2. Avoid requiring `hyperfine`; use `/usr/bin/time` or shell timestamps.
3. Run the script and record expectations in README.
4. Commit: `test: add performance smoke script`.

### Task 4: Final audit and quality fixes

**Objective:** Run full quality gates, fix issues, and keep one final conventional commit if fixes are needed.

**Files:**
- Modify as needed.

**Steps:**
1. Run `cargo fmt --check`; fix formatting with `cargo fmt` if needed.
2. Run `cargo clippy --all-targets -- -D warnings`; fix all warnings.
3. Run `cargo test`.
4. Run `cargo build --release`.
5. Run `scripts/perf-smoke.sh`.
6. Audit implementation against all plan checklists and `docs/mvp-spec.md`.
7. If fixes were required, commit: `fix: resolve mvp audit findings` or a more specific conventional commit.

### Audit checklist

- [ ] README covers install/build, schema, commands, examples, and limitations.
- [ ] CI context capture only records safe allowlisted env vars.
- [ ] Performance smoke test passes.
- [ ] `fmt`, `clippy`, `test`, and release build pass.
- [ ] Implementation matches all docs/plans and MVP spec.
