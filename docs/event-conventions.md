# Browser and Agent Event Conventions

These conventions keep browser QA and agent workflow trails readable without requiring a custom schema for every tool. Producers should keep the core runtrail envelope stable and put tool-specific details in `attrs` and `body`.

## Browser QA Events

Recommended event names:

| Event | Purpose | Suggested body |
| --- | --- | --- |
| `browser.navigate` | Page navigation | `{ "url": "https://example.test", "status": 200 }` |
| `browser.screenshot` | Screenshot/artifact capture | `{ "artifact": ".runtrail/artifacts/home.png", "label": "home page" }` |
| `browser.console` | Console warning/error | `{ "level": "error", "message": "...", "url": "..." }` |
| `browser.accessibility` | Accessibility snapshot | `{ "artifact": ".runtrail/artifacts/a11y.json", "violations": 0 }` |
| `browser.assertion` | Explicit QA assertion | `{ "assertion": "login form is visible", "passed": true }` |

Use `level=error` for failed assertions, uncaught browser exceptions, or network failures that should fail the workflow.

## Agent Tool Events

Recommended event names:

| Event | Purpose | Suggested body |
| --- | --- | --- |
| `agent.tool.start` | Tool invocation started | `{ "tool": "terminal", "input_preview": "cargo test" }` |
| `agent.tool.end` | Tool invocation ended | `{ "tool": "terminal", "success": true, "duration_ms": 1200 }` |
| `agent.model` | Model/provider metadata | `{ "provider": "openai", "model": "gpt-5.5" }` |
| `agent.approval` | Human approval/denial | `{ "action": "push main", "approved": true }` |
| `agent.file_edit` | File edit summary | `{ "path": "src/main.rs", "operation": "patch" }` |
| `agent.artifact` | Generated artifact | `{ "artifact": ".runtrail/artifacts/report.html", "kind": "html" }` |

Never log raw secrets, full prompts containing credentials, or unbounded tool output. Prefer preview fields with redaction/truncation metadata.

## Minimal Browser QA Example

```bash
runtrail log --event browser.navigate \
  --attr url=https://example.test \
  --body '{"status":200}'
runtrail log --event browser.assertion --level error \
  --body '{"assertion":"console has no errors","passed":false,"message":"ReferenceError: app is not defined"}'
runtrail repair-prompt --event browser.assertion --level error
```

## Hermes / PR Review Recipe

1. Log `agent.tool.start` before long-running commands or browser actions.
2. Log `agent.tool.end` with redacted previews and `duration_ms`.
3. Log `repo.snapshot` and `repo.diff --stat-only` before requesting review.
4. Attach artifacts under `.runtrail/artifacts/` and reference paths from JSONL events.
5. Use `runtrail inspect --event agent.tool.end` to skim the run.
