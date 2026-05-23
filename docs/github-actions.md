# GitHub Actions CI Capture

`runtrail ci capture` records a safe, portable fixture for a failing CI job. It keeps JSONL as the source of truth and stores artifact references under `.runtrail/artifacts/`.

```yaml
- name: Capture runtrail context
  if: always()
  run: |
    runtrail ci github-context --file .runtrail/events.jsonl
    runtrail repo snapshot --file .runtrail/events.jsonl
    runtrail repo diff --file .runtrail/events.jsonl --stat-only
    runtrail ci capture --file .runtrail/events.jsonl
    runtrail repair-prompt --file .runtrail/events.jsonl > .runtrail/repair.md

- uses: actions/upload-artifact@v4
  if: always()
  with:
    name: runtrail-fixture
    path: .runtrail/
```

The capture intentionally omits secret values and broad environment dumps. Unsupported local replay gaps such as services, permissions, hosted runner image differences, and matrix differences are emitted as explicit warnings in the `ci.capture` event body.
