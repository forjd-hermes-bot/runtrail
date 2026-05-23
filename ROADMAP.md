# runtrail Roadmap

`runtrail` is a local-first event trail for agentic development workflows: command evidence, repository state, CI context, test results, browser QA steps, and agent notes in one compact JSONL stream.

This roadmap is directional, not a release contract. It records what is already shipped on `main`, what is next, and which longer-term ideas should wait until the core workflow has been dogfooded in real debugging and CI-repair loops.

## Product Principles

- **Evidence over vibes:** every feature should make a future debugging or repair session easier to ground in facts.
- **Local-first and portable:** a trail should be readable with `tail`, `jq`, Git diffs, and normal text tooling.
- **Agent-ready by default:** summaries and repair prompts should be concise enough to paste into an agent while preserving important failure context.
- **Safe capture:** prefer explicit allowlists, bounded previews, truncation markers, and redaction metadata over broad environment or log capture.
- **Small and boring:** no daemon, database, hosted service, or custom query language until the plain-file workflow proves it needs more.
- **Stable core, experimental edges:** keep the event envelope predictable while allowing new event names and payloads to evolve.

## Shipped Baseline

The current `main` branch includes the original MVP plus several post-MVP slices:

- JSONL event storage at `.runtrail/events.jsonl`.
- Core event envelope validation.
- `runtrail log` for appending arbitrary events.
- `runtrail run` for command start/end evidence with bounded stdout/stderr previews.
- Secret-looking output redaction and truncation metadata for command previews.
- Safe command environment metadata capture via explicit allowlists.
- `runtrail repo snapshot` and `runtrail repo diff` for Git context, including normalized remote metadata.
- `runtrail ci github-context` for safe GitHub Actions metadata capture.
- `runtrail ci capture` for portable CI repair fixture creation under `.runtrail/artifacts/`.
- `runtrail tail`, `summarise`, `diff`, and `validate`.
- `runtrail validate --strict` for CI-focused format hardening where `seq` must match the physical JSONL line number.
- `runtrail repair-prompt` for agent-ready failure handoff, with event/level/trace filters.
- `runtrail replay` for conservative command-hint output rather than pretending to be a full CI emulator.
- `runtrail index` and `runtrail inspect` for lightweight trail exploration.
- Browser QA and agent workflow event conventions, with example JSONL logs.
- Schema docs, examples, CI, release automation, binary builds, and installer.

## Recently Completed

These roadmap items have already been implemented and should now be treated as shipped behavior, not pending work:

### Safer, richer capture

- Added first-class redaction helpers for command output previews.
- Added consistent truncated/redacted preview metadata.
- Added allowlisted command environment metadata capture.
- Improved repository snapshots with normalized remote metadata and cleanliness details.
- Added dependency metadata capture for common Rust, Node, and Python project markers.
- Added tests proving obvious secret-looking values are not emitted in command previews.

### CI fixture capture

- Added `runtrail ci capture` for CI-oriented trail creation.
- Captures safe CI context, repository evidence, changed files, and dependency metadata.
- Stores optional repair artifacts under `.runtrail/artifacts/` and references them from the event trail.
- Emits unsupported-feature warnings for local replay gaps such as services, secrets, permissions, hosted runner differences, and matrix differences.

### Replay and repair ergonomics

- Added `runtrail replay` as conservative command-hint output, not a full CI emulator.
- Added replay metadata such as supported/partial/unsupported context.
- Improved repair prompt grouping around failures and repository context.
- Added filters for `repair-prompt` by event, level, and trace ID.
- Kept Markdown output suitable for agent handoffs and issue/PR comments.

### Inspection, indexing, and workflow conventions

- Added lightweight JSON indexes for trail exploration.
- Added `runtrail inspect` for compact human-readable event previews.
- Documented event conventions for browser QA and agent tool workflows.
- Added browser QA and agent session example logs.

## Next Roadmap

### v0.9 — Format hardening and compatibility

Goal: prepare the event format for a stable 1.0 promise.

- Resolve the schema naming/versioning story:
  - keep `cel.v1` for compatibility, or
  - migrate to a `runtrail.v1` schema identifier before 1.0.
- Document producer/consumer compatibility rules for the 1.x line.
- Add migration guidance for any pre-1.0 envelope differences.
- Add golden fixture tests for representative event logs:
  - command run success/failure,
  - CI capture,
  - browser QA,
  - agent session,
  - repair/replay handoff.
- Define how strict validation should evolve beyond `seq == line_number` without breaking normal JSONL workflows.
- Decide whether compact binary export is needed before 1.0 or should remain post-1.0.

### v0.10 — Dogfood and polish

Goal: make runtrail easier to use repeatedly in real agent and CI workflows.

- Add copy-paste GitHub Actions examples for:
  - capturing a failed job,
  - uploading `.runtrail/` artifacts,
  - generating a repair prompt as a workflow artifact.
- Add a performance smoke test for larger trails, such as 100k events.
- Improve docs around safe sharing and artifact review before publishing logs.
- Add more realistic example trails from actual debugging sessions, with sensitive values redacted.
- Decide whether `summarise` should gain the same filters as `repair-prompt`.
- Add clear troubleshooting docs for common malformed-log and CI-capture failure cases.

### v1.0 — Stable local event trail

Goal: provide a stable CLI and schema that other tools can safely produce and consume.

- Commit to a stable JSONL envelope for the 1.x line.
- Keep existing core commands compatible:
  - `log`,
  - `run`,
  - `repo snapshot`,
  - `repo diff`,
  - `ci github-context`,
  - `ci capture`,
  - `tail`,
  - `summarise`,
  - `diff`,
  - `validate`,
  - `repair-prompt`,
  - `replay`,
  - `index`,
  - `inspect`.
- Publish complete schema docs and producer guidance.
- Publish complete installation, release, and upgrade documentation.
- Verify Linux, macOS, and Windows release assets.
- Provide clear compatibility guidance for third-party producers and consumers.

## Suggested Immediate Next PRs

1. **Schema identifier decision**
   - Decide whether the stable schema should remain `cel.v1` or become `runtrail.v1`.
   - Document the compatibility and migration plan.

2. **Golden fixture tests**
   - Add checked-in representative JSONL fixtures under `examples/` or `tests/fixtures/`.
   - Validate them in unit/integration tests and strict mode where appropriate.

3. **GitHub Actions repair workflow docs**
   - Add a workflow snippet that captures `.runtrail/` on failure.
   - Show how to upload trail artifacts and generate a repair prompt.

4. **Large-log performance smoke**
   - Add a script or test path for validating and summarising large synthetic trails.
   - Document expected local performance bounds.

5. **Safe sharing guide**
   - Add a short checklist for reviewing trails before sharing in issues, PRs, or with agents.
   - Cover secrets, proprietary diffs, sensitive paths, and artifact retention.

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
- Should compact binary export exist before 1.0, or stay deferred until JSONL limitations are proven?
- How much output preview is useful before logs become too large or risky?
- Which integrations should be first-class examples beyond Hermes, GitHub Actions, browser QA, and generic shell workflows?
- Should indexes live inside `.runtrail/` by default or be generated on demand into a cache path?
- Should repair summaries grow PR-comment output directly, or should that stay as integration glue outside the core CLI?
