# CI Failure Evidence and Repair Prompt Workflow Research

## Summary

The adjacent CI-fixture idea informs `compact-event-log`: CI failures become more useful for agents when workflow metadata, environment expectations, failing logs, changed files, dependency metadata, and safe local replay commands are captured in a portable bundle. `compact-event-log` should provide the event trail that such a bundle can embed or reference.

## Sources

- GitHub Actions contexts: <https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/accessing-contextual-information-about-workflow-runs>
- GitHub Actions environment variables: <https://docs.github.com/en/actions/reference/variables-reference>
- GitHub Actions workflow commands: <https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions>
- GitHub Actions events: <https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows>
- GitHub webhooks: <https://docs.github.com/en/webhooks/webhook-events-and-payloads>
- `act`: <https://github.com/nektos/act>
- Cargo metadata: <https://doc.rust-lang.org/cargo/commands/cargo-metadata.html>
- Cargo lockfile: <https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html>

## CI evidence worth recording

A useful CI event stream should capture enough context for humans and coding agents to understand what failed without fetching the full CI platform again.

Recommended evidence categories:

- Workflow identity: name, path, job, step, runner OS.
- Trigger: event name, ref, SHA, PR number, actor.
- Run identity: run ID, run number, run attempt.
- Toolchain: Rust version, Cargo version, Node version if present.
- Dependency state: `Cargo.lock`, `cargo metadata` summary, package versions.
- Command evidence: command line, cwd, environment allowlist, exit code, duration.
- Failure evidence: stderr/stdout previews, annotations, failing test names, log artifact refs.
- Repo state: branch, HEAD, dirty files, status summary.
- Replay hints: `act` command when supported, shell fallback commands when not.

## GitHub Actions-specific fields

Useful environment/context fields:

- `GITHUB_WORKFLOW`
- `GITHUB_RUN_ID`
- `GITHUB_RUN_NUMBER`
- `GITHUB_RUN_ATTEMPT`
- `GITHUB_JOB`
- `GITHUB_ACTION`
- `GITHUB_ACTOR`
- `GITHUB_EVENT_NAME`
- `GITHUB_REF`
- `GITHUB_SHA`
- `GITHUB_REPOSITORY`
- `RUNNER_OS`
- `RUNNER_ARCH`

These map cleanly into `attrs`:

```json
{
  "attrs": {
    "github.workflow": "CI",
    "github.run_id": "123456",
    "github.run_attempt": "1",
    "github.job": "test",
    "github.event_name": "pull_request",
    "github.repository": "owner/repo",
    "git.sha": "abc123",
    "runner.os": "Linux"
  }
}
```

## Workflow commands and annotations

GitHub supports annotations such as `::error file=app.js,line=1::Missing semicolon`, grouped logs, environment files, outputs, and `GITHUB_STEP_SUMMARY`. `compact-event-log` can record these as structured events:

- `ci.annotation.error`
- `ci.annotation.warning`
- `ci.summary`
- `ci.output`

## Local replay with `act`

`act` can run GitHub Actions locally using Docker, but not every hosted-runner feature, service, permission, secret, or environment matches GitHub. A repair workflow should emit clear warnings when local replay is approximate.

Design implications:

- Store replay commands as hints, not guarantees.
- Separate `replay.supported=true/false/partial` metadata.
- Preserve unsupported feature warnings in the event log.

Example event:

```json
{
  "v": 1,
  "seq": 44,
  "ts": "2026-05-22T13:00:00Z",
  "event": "ci.replay.hint",
  "attrs": {"tool": "act", "support": "partial"},
  "body": {
    "command": "act pull_request -j test",
    "warnings": ["GitHub-hosted OIDC credentials are not reproduced locally"]
  }
}
```

## Repair prompt generation

Agent-ready repair prompts should be evidence-first and avoid unsafe commands. A generated prompt can include:

1. Repository and CI run identity.
2. Failing command and exit code.
3. Minimal failing log excerpt.
4. Changed files and dependency metadata.
5. Suspected causes based on error patterns.
6. Safe commands to try locally.
7. Explicit constraints: do not push secrets, do not rewrite unrelated files, keep changes minimal.

`compact-event-log summarise` can output a Markdown summary that later becomes part of such a prompt.

## MVP implications for compact-event-log

The event-log MVP should include enough primitives to support CI evidence bundles later:

- Event envelope with `event`, `attrs`, and `body`.
- Command result events with `cmd`, `cwd`, `exit_code`, `duration_ms`, and output previews.
- Repo status/change events.
- CI context capture from environment variables.
- Markdown summary generation grouped by event type and failures.
- Diff between two logs to show newly introduced failures or changed files.

## Safety and redaction

CI logs often contain secrets. The MVP should include basic redaction:

- Redact common token-like env names: `TOKEN`, `SECRET`, `PASSWORD`, `KEY`, `AUTH`.
- Truncate large outputs.
- Store hashes of full outputs when useful.
- Make redaction explicit in event attributes.
