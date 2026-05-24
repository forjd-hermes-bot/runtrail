# GitHub Actions Repair Workflow

`runtrail ci capture` records a safe, portable fixture for a failing GitHub Actions job. It keeps JSONL as the source of truth, stores artifact references under `.runtrail/artifacts/`, and generates a Markdown repair prompt that can be handed to an agent or attached to an issue.

The goal is not to emulate GitHub Actions locally. The goal is to package enough evidence that a human or agent can start from facts instead of “CI failed”.

## Minimal failure-capture steps

Use this pattern when your workflow already runs tests or builds and you only want to capture context after the job has run:

```yaml
- name: Capture runtrail repair context
  if: failure()
  run: |
    mkdir -p .runtrail
    runtrail ci github-context --file .runtrail/events.jsonl
    runtrail repo snapshot --file .runtrail/events.jsonl
    runtrail repo diff --file .runtrail/events.jsonl
    runtrail ci capture --file .runtrail/events.jsonl
    runtrail repair-prompt --file .runtrail/events.jsonl > .runtrail/repair.md

- name: Upload runtrail repair fixture
  if: failure()
  uses: actions/upload-artifact@v4
  with:
    name: runtrail-repair-${{ github.run_id }}-${{ github.run_attempt }}
    path: |
      .runtrail/events.jsonl
      .runtrail/repair.md
    if-no-files-found: error
    retention-days: 7
```

Use `if: failure()` for normal jobs so successful runs do not upload artifacts. Use `if: always()` only when you explicitly want a trail for both success and failure.

## Complete Rust workflow example

This example installs `runtrail`, records the test command with bounded stdout/stderr previews, generates a repair prompt on failure, and uploads enumerated repair files as an artifact.

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        run: |
          rustup toolchain install stable --profile minimal
          rustup default stable

      - name: Install runtrail
        run: |
          curl -fsSL https://raw.githubusercontent.com/forjd/runtrail/main/install.sh \
            | RUNTRAIL_INSTALL_TAG=runtrail-v0.3.0 bash
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"

      - name: Run tests with runtrail evidence
        run: |
          runtrail ci github-context --file .runtrail/events.jsonl
          runtrail repo snapshot --file .runtrail/events.jsonl
          runtrail run --file .runtrail/events.jsonl -- cargo test

      - name: Capture repair fixture
        if: failure()
        run: |
          runtrail repo diff --file .runtrail/events.jsonl
          runtrail ci capture --file .runtrail/events.jsonl
          runtrail repair-prompt --file .runtrail/events.jsonl > .runtrail/repair.md
          runtrail validate --file .runtrail/events.jsonl --strict

      - name: Upload runtrail repair fixture
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: runtrail-repair-${{ github.run_id }}-${{ github.run_attempt }}
          path: |
            .runtrail/events.jsonl
            .runtrail/repair.md
          if-no-files-found: error
          retention-days: 7
```

## What the artifact contains

A repair fixture normally contains:

- `.runtrail/events.jsonl` — canonical event trail.
- `.runtrail/repair.md` — Markdown prompt for humans or agents.
- `.runtrail/artifacts/` — optional derived artifacts and metadata from `runtrail ci capture`; upload individual files from this directory only after review.

The trail may include:

- safe GitHub Actions metadata from `runtrail ci github-context`;
- repository branch, HEAD, dirty state, and changed files;
- command start/end events with bounded stdout/stderr previews;
- dependency metadata for common Rust, Node, and Python project markers;
- explicit warnings for local replay gaps such as services, secrets, permissions, hosted runner image differences, and matrix differences.

## Safety notes

`runtrail` avoids broad secret capture by default, but you should still review artifacts before posting them publicly.

Before sharing a fixture outside your trusted team, check for:

- proprietary source snippets in repo diffs;
- sensitive file paths;
- secrets printed by commands before redaction catches them;
- credentials or tokens inside application logs;
- oversized artifacts that are not useful for repair.

Prefer the default stat-only `runtrail repo diff` in public or semi-public workflows. Capture full patches with `runtrail repo diff --patch` only when the artifact stays within a trusted boundary.

## Handing the prompt to an agent

Download the artifact, then use the generated prompt directly:

```bash
unzip runtrail-repair-*.zip -d runtrail-repair
sed -n '1,220p' runtrail-repair/.runtrail/repair.md
```

If you want to inspect the raw trail first:

```bash
runtrail validate --file runtrail-repair/.runtrail/events.jsonl --strict
runtrail inspect --file runtrail-repair/.runtrail/events.jsonl --lines 20
runtrail replay --file runtrail-repair/.runtrail/events.jsonl
```

## Troubleshooting

- **`runtrail: command not found`** — make sure the install step writes `~/.local/bin` to `$GITHUB_PATH`, or install to a directory already on `PATH`.
- **`validate --strict` fails** — a step may have edited or concatenated the JSONL file. Preserve one event per line and avoid manual edits.
- **No artifact uploaded** — the capture/upload steps probably used `if: failure()` on a successful job. Use `if: always()` when you want artifacts for all outcomes.
- **Repair prompt is too sparse** — wrap the failing command with `runtrail run -- ...` so stdout/stderr previews and exit status are captured.
- **Replay is incomplete** — expected. `runtrail replay` emits conservative hints and warnings; it is not a full hosted-runner emulator.
