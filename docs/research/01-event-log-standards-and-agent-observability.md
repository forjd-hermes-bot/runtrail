# Event Log Standards and Agent Observability Research

## Summary

`compact-event-log` should use a JSONL-first append-only stream with a small, stable event envelope. The strongest external alignment points are JSON Lines/NDJSON for framing, event sourcing for immutability and replay, OpenTelemetry for structured observability vocabulary, W3C Trace Context for correlation, CloudEvents for portable event-envelope concepts, GitHub Actions contexts for CI evidence, and LLM observability platforms for agent/tool-call event shapes.

## Sources

- JSON Lines: <https://jsonlines.org/>
- NDJSON spec: <https://github.com/ndjson/ndjson-spec>
- Martin Fowler, Event Sourcing: <https://martinfowler.com/eaaDev/EventSourcing.html>
- OpenTelemetry Logs Data Model: <https://opentelemetry.io/docs/specs/otel/logs/data-model/>
- W3C Trace Context: <https://www.w3.org/TR/trace-context/>
- CloudEvents: <https://cloudevents.io/> and <https://github.com/cloudevents/spec>
- GitHub Actions events: <https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows>
- GitHub Actions contexts: <https://docs.github.com/en/actions/writing-workflows/choosing-what-your-workflow-does/accessing-contextual-information-about-workflow-runs>
- GitHub webhooks: <https://docs.github.com/en/webhooks/webhook-events-and-payloads>
- GitHub workflow commands: <https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions>
- Langfuse tracing: <https://langfuse.com/docs/tracing>
- Phoenix tracing: <https://arize.com/docs/phoenix/tracing/llm-traces-1>

## JSONL / NDJSON

JSON Lines requires UTF-8, one valid JSON value per line, and newline termination. NDJSON similarly requires newline-delimited RFC 8259 JSON texts, with no raw newlines inside a record. These properties fit an append-only developer event stream because records can be appended, grepped, concatenated, streamed, and recovered if the final line is incomplete.

Design implications:

- Use one compact JSON object per line.
- Always write a trailing newline.
- Reject non-object records in validation.
- Keep records single-line and compact by default.
- Make `.jsonl` the canonical extension while documenting NDJSON compatibility.
- Support external compression later rather than making it part of the MVP.

## Event sourcing

Event sourcing stores state changes as a sequence of immutable events and allows state to be rebuilt by replay. It supports temporal query, event replay, and complete rebuilds, but long logs may later need snapshots or indexes.

Design implications:

- Events are append-only and immutable.
- The log is the source of truth; summaries and indexes are derived.
- Include enough causality and source metadata to reconstruct what happened.
- Use `seq` for deterministic ordering when timestamps collide.
- Defer snapshots to a later version.

## OpenTelemetry logs

OpenTelemetry log records include timestamp, observed timestamp, trace ID, span ID, severity, body, attributes, resource, instrumentation scope, and event name. `compact-event-log` does not need to implement OTLP, but the vocabulary is useful.

Recommended compact mapping:

| compact-event-log | OpenTelemetry concept |
| --- | --- |
| `ts` | `Timestamp` |
| `event` | `EventName` |
| `level` | `SeverityText` |
| `trace_id` | `TraceId` |
| `span_id` | `SpanId` |
| `attrs` | `Attributes` |
| `body` | `Body` |

## W3C Trace Context

Trace Context standardizes propagation of `trace-id`, `parent-id`, and flags. Agent workflows cross process, browser, CI, and repository boundaries, so optional trace fields are useful even in a local file format.

Design implications:

- Add optional `trace_id`, `span_id`, and `parent_span_id` fields.
- Use lowercase hex IDs compatible with common tracing systems.
- A run can be a trace; actions and tool calls can be spans/events.

## CloudEvents

CloudEvents defines a portable envelope with fields such as `id`, `source`, `specversion`, `type`, `time`, `subject`, and `data`. It is vendor-neutral but verbose for the default format.

Design implications:

- Borrow concepts, not the full default field names.
- Use `id`, `src`, `event`, `ts`, `subject`, and `body`.
- Consider future export to CloudEvents.

## GitHub Actions and CI evidence

GitHub Actions exposes contextual data including `github.event`, `github.run_id`, `github.run_number`, `github.run_attempt`, `github.sha`, `github.workflow`, `github.actor`, and job/step identifiers. Webhooks include `X-GitHub-Event`, `X-GitHub-Delivery`, and optional signatures. Workflow commands support annotations and step summaries.

Design implications:

- Provide CI-friendly event kinds such as `ci.workflow`, `ci.job`, `ci.step`, and `ci.result`.
- Normalize GitHub fields in `attrs`: `github.run_id`, `github.run_attempt`, `git.sha`, `github.workflow`, `github.actor`.
- Store large logs or payloads as artifact references rather than inline by default.

## Agent observability

LLM observability tools converge on traces, sessions, observations/spans, tool calls, inputs/outputs, latency, costs, and metadata. A local JSONL event stream can serve as a cheap agent black-box recorder.

Useful event kinds:

- `agent.session.start`
- `agent.session.end`
- `agent.turn.start`
- `agent.turn.end`
- `agent.llm.call`
- `agent.llm.result`
- `agent.tool.start`
- `agent.tool.end`
- `agent.tool.error`
- `browser.step`
- `browser.assert`
- `repo.change`
- `repo.commit`
- `ci.result`

Privacy implications:

- Prefer previews and hashes for large command outputs.
- Redact secrets before writing event bodies.
- Include `attrs.redacted=true` when content is intentionally truncated or scrubbed.

## Recommended v0 envelope

```json
{
  "v": 1,
  "id": "01HYEXAMPLE",
  "seq": 12,
  "ts": "2026-05-22T12:34:56.789Z",
  "level": "info",
  "event": "agent.tool.end",
  "src": "hermes-agent",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "parent_span_id": "7b34da6a3ce929d",
  "attrs": {
    "tool.name": "terminal",
    "status": "ok"
  },
  "body": {
    "cmd": "cargo test",
    "exit_code": 0
  }
}
```
