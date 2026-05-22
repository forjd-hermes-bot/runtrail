# Schema v1

`runtrail` stores newline-delimited JSON (`.jsonl`). Each line is one independent event object. The format is intentionally append-only, grep-able, and safe to copy into agent repair prompts.

## Envelope

| Field | Type | Required | Description |
|---|---:|---:|---|
| `schema` | string | yes | Current schema version. MVP writes `cel.v1`. |
| `id` | string | yes | ULID for stable identity across copied logs. |
| `seq` | integer | yes | 1-based sequence number within the file. |
| `ts` | string | yes | RFC3339 UTC timestamp. |
| `event` | string | yes | Dot-separated event name such as `command.end`. |
| `level` | string | yes | `trace`, `debug`, `info`, `warn`, or `error`. |
| `src` | string | no | Producer/source, e.g. `runtrail`, `git`, `github-actions`. |
| `trace_id` | string | no | 32 lowercase hex characters. |
| `span_id` | string | no | 16 lowercase hex characters. |
| `parent_span_id` | string | no | 16 lowercase hex characters. |
| `duration_ms` | integer | no | Duration in milliseconds. |
| `attrs` | object | yes | Small indexed metadata. |
| `body` | any JSON | yes | Event-specific payload. |

## Core event names

### `agent.note`

Freeform note from a human or agent.

```json
{"schema":"cel.v1","id":"01HX...","seq":1,"ts":"2026-05-22T10:12:00Z","event":"agent.note","level":"info","src":"runtrail","attrs":{},"body":{"message":"Failure likely due to missing mocked env var"}}
```

### `command.start`

Emitted by `runtrail run` before the child command output is recorded.

Body fields:

- `cmd`: array of command/argument strings
- `cwd`: working directory
- `started_at`: RFC3339 timestamp

### `command.end`

Emitted by `runtrail run` after the child exits.

Body fields:

- `cmd`: array of command/argument strings
- `cwd`: working directory
- `exit_code`: process exit code, or `128` if terminated without a code
- `success`: boolean
- `duration_ms`: elapsed command time
- `stdout_preview`: UTF-8 lossy stdout preview
- `stderr_preview`: UTF-8 lossy stderr preview
- `stdout_truncated`: boolean
- `stderr_truncated`: boolean

### `repo.snapshot`

Emitted by `runtrail repo snapshot`.

Body fields:

- `repo_root`
- `branch`
- `head`
- `dirty`
- `files`: array of `{ "path", "status" }` entries from `git status --porcelain`

### `repo.diff`

Emitted by `runtrail repo diff`.

Body fields:

- `repo_root`
- `branch`
- `head`
- `dirty`
- `stat`: `git diff --stat`
- `patch`: `git diff --patch`, or `null` when `--stat-only` is used

### `ci.github.context`

Emitted by `runtrail ci github-context` using a strict allowlist of GitHub Actions environment variables. Secrets and arbitrary environment variables are intentionally not captured.

## Compatibility rules

- Readers should ignore unknown top-level fields.
- Producers should keep `schema: "cel.v1"` until a breaking envelope change is required.
- Event-specific `body` payloads may grow additively.
- `attrs` should remain small and mostly scalar; large evidence belongs in `body`.
- Secrets must not be written into logs. Prefer allowlists over denylists.
