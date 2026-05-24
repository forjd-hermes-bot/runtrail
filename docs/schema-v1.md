# Schema v1

`runtrail` stores newline-delimited JSON (`.jsonl`). Each line is one independent event object. The format is intentionally append-only, grep-able, and safe to copy into agent repair prompts.

The current schema identifier is `runtrail.v1`. Earlier design notes used an integer `v` field or the experimental `cel.v1` name; those pre-1.0 forms are not accepted by current validators.

## Envelope

| Field | Type | Required | Description |
|---|---:|---:|---|
| `schema` | string | yes | Current schema identifier. Must be `runtrail.v1`. |
| `id` | string | yes | 26-character ULID for stable identity across copied logs. |
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
{"schema":"runtrail.v1","id":"01HX...","seq":1,"ts":"2026-05-22T10:12:00Z","event":"agent.note","level":"info","src":"runtrail","attrs":{},"body":{"message":"Failure likely due to missing mocked env var"}}
```

### `command.start`

Emitted by `runtrail run` before the child command is spawned, so hangs and spawn failures still leave lifecycle evidence.

Body fields:

- `cmd`: array of command/argument strings
- `cwd`: working directory
- `started_at`: RFC3339 timestamp

### `command.end`

Emitted by `runtrail run` after the child exits.

Body fields:

- `cmd`: array of command/argument strings
- `cwd`: working directory
- `exit_code`: process exit code, `128 + signal` on Unix signal termination, or `127` for spawn failures
- `success`: boolean
- `spawn_error`: optional spawn failure message
- `duration_ms`: elapsed command time
- `stdout_preview`: UTF-8 lossy stdout preview
- `stderr_preview`: UTF-8 lossy stderr preview
- `stdout_truncated`: boolean
- `stderr_truncated`: boolean
- `stdout_redacted`: boolean
- `stderr_redacted`: boolean

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
- `stat`: combined unstaged and staged diff stats
- `patch`: combined unstaged and staged patches, or `null` unless `runtrail repo diff --patch` is used
- `unstaged`: object containing `stat` and optional `patch`
- `staged`: object containing `stat` and optional `patch`

### `ci.github.context`

Emitted by `runtrail ci github-context` using a strict allowlist of GitHub Actions environment variables. Secrets and arbitrary environment variables are intentionally not captured.

### `ci.capture`

Emitted by `runtrail ci capture` to create a portable repair fixture with safe CI context, repository evidence, dependency metadata, artifact references, and local replay caveats.

## Compatibility rules

- Producers must write `schema: "runtrail.v1"` for the current schema.
- Readers should ignore unknown top-level fields.
- Event-specific `body` payloads may grow additively.
- `attrs` should remain small and mostly scalar; large evidence belongs in `body` or referenced artifacts.
- Required fields are strict in v1: `level`, `attrs`, and `body` must be present even when they are `"info"`, `{}`, or another empty JSON value.
- `runtrail` applies best-effort redaction to command argv, attrs, stdout/stderr previews, and string values in event bodies. Producers should still prefer allowlists over denylists and avoid writing secrets.
- Strict validation additionally requires `seq` to match the physical JSONL line number.

## Migration notes

Pre-1.0 notes and examples may mention either `v: 1` or `schema: "cel.v1"`. Current producers should emit only `schema: "runtrail.v1"`; current validators reject the older identifiers so incompatible logs fail loudly instead of being treated as stable 1.x data.
