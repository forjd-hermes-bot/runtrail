# Deterministic Architecture, Compact Serialization, and CLI Prior Art Research

## Summary

The project should stay JSONL-first for MVP while borrowing deterministic event-stream ideas from NautilusTrader, future compact binary concepts from FlatBuffers, repository metadata patterns from Git, structured-state deltas from JSON Patch, and CLI interoperability principles from tools such as `jq`, `lnav`, `jless`, `angle-grinder`, and `sqlite-utils`.

## Sources

- NautilusTrader GitHub: <https://github.com/nautechsystems/nautilus_trader>
- NautilusTrader docs: <https://nautilustrader.io/docs/latest/>
- FlatBuffers: <https://flatbuffers.dev/>
- FlatBuffers schema evolution: <https://flatbuffers.dev/evolution/>
- FlatBuffers Rust: <https://flatbuffers.dev/languages/rust/>
- Git objects: <https://git-scm.com/book/en/v2/Git-Internals-Git-Objects>
- `git status --porcelain`: <https://git-scm.com/docs/git-status#_porcelain_format_version_2>
- `git diff --raw`: <https://git-scm.com/docs/git-diff#_raw_output_format>
- JSON Patch RFC 6902: <https://datatracker.ietf.org/doc/html/rfc6902>
- JSON Pointer RFC 6901: <https://datatracker.ietf.org/doc/html/rfc6901>
- JSON Merge Patch RFC 7396: <https://datatracker.ietf.org/doc/html/rfc7396>
- Rust `json-patch`: <https://docs.rs/json-patch/latest/json_patch/>
- lnav: <https://lnav.org/features>
- jq: <https://github.com/jqlang/jq>
- jless: <https://github.com/PaulJuliusMartinez/jless>
- angle-grinder: <https://github.com/rcoh/angle-grinder>
- sqlite-utils: <https://github.com/simonw/sqlite-utils>

## NautilusTrader and deterministic event streams

NautilusTrader is a production-grade Rust-native trading engine built around deterministic event-driven architecture. Its docs emphasize message bus patterns, immutable messages, event streams, caches/state, and replayable backtesting/live flows.

Relevant concepts:

- Loose coupling through a message bus.
- Message categories such as data, events, and commands.
- Immutable message integrity after creation.
- Deterministic ordering for simulation and replay.
- Separation between event stream and derived state/cache.

Design implications:

- Treat `compact-event-log` as a black-box recorder and not a mutable state database.
- Include stable ordering: `seq` plus timestamp.
- Include source and actor metadata.
- Include causality links: `parent_id`, trace/span fields, or both.
- Keep derived summaries disposable.

Recommended event categories:

- `run.*`
- `agent.*`
- `tool.*`
- `browser.*`
- `repo.*`
- `ci.*`
- `artifact.*`
- `state.*`
- `note`
- `error`

## FlatBuffers

FlatBuffers offers compact binary serialization with zero-copy-style reads and schema evolution through tables. It is valuable for high-volume or long-running sessions, but binary-first logs are worse for early developer workflows because they are not grepable, easy to diff, or inspectable without generated tooling.

Design implications:

- Do not make FlatBuffers part of MVP storage.
- Keep the logical schema capable of future binary export.
- If added later, use FlatBuffers tables for schema evolution.
- Keep JSONL as the debug/interchange format even if `.celb` archives are added.

Possible future layering:

```text
events.jsonl        canonical append-only event stream
events.idx          optional offset/time/event index
events.celb         optional compact binary archive/export
artifacts/          optional blobs, screenshots, patches, traces
```

## Git metadata and file changes

Git is content-addressed and has stable script-friendly formats for repository state. Porcelain status is explicitly intended for scripts; raw diff includes old/new modes, object IDs, status, and paths.

Design implications:

- Mirror Git vocabulary where possible.
- Support `repo.snapshot`, `repo.change`, and `repo.commit` events.
- Store compact metadata inline and large patches as artifact refs.
- Avoid requiring a Git repo for all logs.

Useful file-change fields:

- `repo_root`
- `head`
- `branch`
- `dirty`
- `path`
- `old_path`
- `status`
- `old_oid`
- `new_oid`
- `old_mode`
- `new_mode`
- `additions`
- `deletions`
- `binary`
- `patch_ref`

Example:

```json
{
  "v": 1,
  "seq": 17,
  "ts": "2026-05-22T12:00:00Z",
  "event": "repo.change",
  "body": {
    "repo": "/home/dan/project",
    "head": "abc123",
    "branch": "main",
    "files": [
      {"path": "src/lib.rs", "status": "M", "additions": 8, "deletions": 2}
    ]
  }
}
```

## JSON Patch and structured state changes

JSON Patch defines explicit operations (`add`, `remove`, `replace`, `move`, `copy`, `test`) using JSON Pointer paths. Merge Patch is simpler but has ambiguous null semantics and is less precise for arrays.

Design implications:

- Use JSON Patch for structured state deltas.
- Use Git/unified diffs for source-code text changes.
- Keep state patch payloads optional and event-specific.

Example:

```json
{
  "v": 1,
  "seq": 29,
  "ts": "2026-05-22T12:10:00Z",
  "event": "state.patch",
  "body": {
    "target": "browser.session",
    "patch": [
      {"op": "replace", "path": "/url", "value": "https://example.com/dashboard"}
    ]
  }
}
```

## Existing CLI log tools

The ecosystem already has strong tools for JSON and logs: `jq` for JSON filtering, `lnav` for log viewing, `jless` for interactive JSON, `angle-grinder` for log slicing/aggregation, and `sqlite-utils` for loading JSONL into SQLite.

Design implications:

- Embrace interoperability; do not invent a query language in MVP.
- Keep output valid JSONL so it works with `jq`, `lnav`, and SQLite importers.
- First-party CLI should focus on project-specific workflows: `log`, `tail`, `summarise`, `diff`, `validate`.

Useful pipeline examples:

```bash
jq 'select(.event == "repo.change")' events.jsonl
lnav events.jsonl
sqlite-utils insert logs.db events events.jsonl --nl
```

## Recommended MVP design principles

1. Append-only by default.
2. One event per line.
3. Every line is a JSON object.
4. Stable envelope, extensible `attrs` and `body`.
5. Deterministic ordering via `seq`.
6. Optional trace/span correlation.
7. Optional content hashing later.
8. External artifacts for large blobs.
9. Streaming reads and writes.
10. No custom query language in v0.
