# runtrail Roadmap

`runtrail` is a local-first event trail for agentic development workflows: command evidence, repository state, CI context, test results, browser QA steps, and agent notes in one compact JSONL stream.

This roadmap is directional, not a release contract. It captures the next useful product slices after the MVP and should change as the tool gets dogfooded in real debugging and CI-repair loops.

## Product Principles

- **Evidence over vibes:** every feature should make a future debugging or repair session easier to ground in facts.
- **Local-first and portable:** a trail should be readable with `tail`, `jq`, Git diffs, and normal text tooling.
- **Agent-ready by default:** summaries and repair prompts should be concise enough to paste into an agent while preserving the important failure context.
- **Safe capture:** prefer explicit allowlists, bounded previews, truncation markers, and redaction metadata over broad environment or log capture.
- **Small and boring:** no daemon, database, hosted service, or custom query language until the plain-file workflow proves it needs more.
- **Stable core, experimental edges:** keep the event envelope predictable while allowing new event names and payloads to evolve.

## Current Baseline

The MVP already includes:

- JSONL event storage at `.runtrail/events.jsonl`.
- Core event envelope validation.
- `runtrail log` for appending arbitrary events.
- `runtrail run` for command start/end evidence.
- `runtrail repo snapshot` and `runtrail repo diff` for Git context.
- `runtrail ci github-context` for safe GitHub Actions metadata capture.
- `runtrail tail`, `summarise`, `diff`, and `validate`.
- `runtrail repair-prompt` for agent-ready failure handoff.
- Schema docs, examples, CI, release automation, binary builds, and installer.

## Near-Term Roadmap

### v0.4 — Safer, richer capture

Goal: make the evidence captured by `runtrail` more useful without making logs noisy or risky.

- Add first-class redaction helpers for command output previews.
- Mark truncated and redacted fields consistently in `attrs` or event bodies.
- Capture selected command environment metadata via allowlists, never broad env dumps.
- Improve `repo snapshot` with remote URL normalization, upstream branch, and commit cleanliness details.
- Add optional dependency metadata events for common ecosystems:
  - Rust: `Cargo.toml`, `Cargo.lock`, toolchain/channel.
  - Node: `package.json`, lockfile type, package manager.
  - Python: `pyproject.toml`, lockfile/requirements presence.
- Add tests that prove obvious secret-looking values are not emitted by default.

### v0.5 — CI fixture capture

Goal: turn a CI failure into a portable local bundle an agent can reason about.

- Add `runtrail ci capture` for CI-oriented trail creation.
- Capture workflow/job identity, safe GitHub context, failing command summaries, changed files, and dependency metadata.
- Store optional artifacts under `.runtrail/artifacts/` with references from JSONL events.
- Add explicit unsupported-feature warnings for local replay gaps such as services, secrets, permissions, hosted runner differences, and matrix differences.
- Produce a single repair handoff containing:
  - failure summary,
  - relevant commands,
  - repo diff/stat,
  - safe reproduction hints,
  - suspected missing context.
- Document GitHub Actions usage patterns and a copy-paste workflow snippet.

### v0.6 — Replay and repair ergonomics

Goal: make the trail actionable after capture.

- Add `runtrail replay` as a conservative command-hint runner, not a magic CI emulator.
- Support replay metadata such as `supported`, `partial`, and `unsupported_reason`.
- Generate shell fallback commands when `act` or platform-specific runners are unavailable.
- Improve `repair-prompt` grouping by failing command, changed file, CI job, and latest repo state.
- Add `--since`, `--event`, `--level`, and `--trace-id` filters to summary/repair workflows.
- Add Markdown output that is stable enough for PR comments or issue comments.

### v0.7 — Inspection and indexing

Goal: keep JSONL as the source of truth while making larger trails pleasant to inspect.

- Add lightweight derived indexes for event ID, event name, level, timestamp, and trace ID.
- Keep indexes optional and rebuildable; never make them required to read a trail.
- Add query-style filters without inventing a full custom query language.
- Add `runtrail inspect` or improve `tail` for compact human-readable event previews.
- Explore `lnav`/SQLite-friendly export paths for larger debugging sessions.
- Add performance smoke tests for larger logs, such as 100k events.

### v0.8 — Browser and agent workflow events

Goal: make runtrail useful beyond shell commands and CI.

- Document event conventions for browser QA:
  - navigation,
  - screenshot/artifact creation,
  - console errors,
  - accessibility snapshots,
  - assertions.
- Document event conventions for agent tool calls:
  - tool start/end,
  - model/provider metadata,
  - approval decisions,
  - file edits,
  - generated artifacts.
- Add examples that mirror real agent sessions and web-app QA loops.
- Add integration recipes for Hermes, browser automation, and GitHub PR review workflows.

### v0.9 — Format hardening

Goal: prepare the event format for a stable 1.0 promise.

- Resolve the schema naming/versioning story and document compatibility rules clearly.
- Add migration guidance for any pre-1.0 envelope differences.
- Add golden tests for representative event logs.
- `runtrail validate --strict` for CI-focused format hardening.
- Decide whether compact binary export is needed before 1.0 or should remain post-1.0.
- Document producer/consumer compatibility expectations.

### v1.0 — Stable local event trail

Goal: provide a stable CLI and schema that other tools can safely produce and consume.

- Commit to the stable JSONL envelope for the 1.x line.
- Keep existing core commands compatible:
  - `log`,
  - `run`,
  - `repo snapshot`,
  - `repo diff`,
  - `ci github-context`,
  - `tail`,
  - `summarise`,
  - `diff`,
  - `validate`,
  - `repair-prompt`.
- Publish complete schema docs and example logs.
- Publish installation and release documentation.
- Verify Linux, macOS, and Windows release assets.
- Provide clear guidance for third-party producers.

## Later Ideas

These are intentionally deferred until the core workflow proves itself.

- Compact binary export for long-running sessions or constrained devices.
- CloudEvents/OpenTelemetry export bridges.
- Rich HTML reports for CI artifacts.
- PR-comment generation from repair summaries.
- GitHub Action wrapper around `runtrail ci capture`.
- TUI or web viewer for local trails.
- Cross-run comparison reports for flaky tests.
- Artifact retention policies and cleanup commands.
- Signed trails or tamper-evident hashes for higher-trust audit workflows.

## Non-Goals

- No hosted service requirement.
- No always-on daemon for the core workflow.
- No broad secret/environment capture.
- No full CI emulator in the core CLI.
- No custom query language while standard tools and simple filters are enough.
- No binary-only storage format; JSONL remains the canonical source of truth through 1.0.

## Open Questions

- Should the schema version remain `cel.v1` for compatibility, or move to a `runtrail.v1` name before 1.0?
- Should replay be a separate command family or part of CI fixture capture?
- How much output preview is useful before logs become too large or risky?
- Which integrations should be first-class examples: Hermes, GitHub Actions, browser QA, or generic shell workflows?
- Should indexes live inside `.runtrail/` by default or be generated on demand into a cache path?

## Suggested Immediate Next PRs

1. **Redaction and truncation metadata**
   - Add explicit redaction/truncation markers to command output previews.
   - Add tests for secret-looking values.

2. **CI fixture capture skeleton**
   - Add a `runtrail ci capture` command that emits safe context and repo evidence.
   - Document a GitHub Actions snippet.

3. **Repair prompt filters**
   - Add `--since`, `--event`, and `--level` filters to `repair-prompt` and `summarise`.

4. **Schema cleanup before 1.0**
   - Decide and document the schema identifier strategy.
   - Add golden fixture tests for example logs.
