#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --release --locked >/dev/null
BIN="$ROOT/target/release/runtrail"
TMP="${TMPDIR:-/tmp}/runtrail-perf-$$"
mkdir -p "$TMP"
LOG="$TMP/events.jsonl"
APPEND_LOG="$TMP/append-events.jsonl"
N="${1:-10000}"
APPEND_N="${RUNTRAIL_APPEND_SMOKE_EVENTS:-200}"

start_ns() { date +%s%N; }
elapsed_ms() {
  local start="$1"
  local end
  end="$(start_ns)"
  echo $(( (end - start) / 1000000 ))
}

start="$(start_ns)"
for i in $(seq 1 "$N"); do
  printf '{"schema":"runtrail.v1","id":"01H00000000000000000000000","seq":%s,"ts":"2026-05-22T12:00:00Z","event":"command.run","level":"info","attrs":{"exit_code":0},"body":{"cmd":"echo %s"}}\n' "$i" "$i"
done > "$LOG"
gen_ms="$(elapsed_ms "$start")"

start="$(start_ns)"
for i in $(seq 1 "$APPEND_N"); do
  "$BIN" log --file "$APPEND_LOG" --event command.run --attr exit_code=0 --body "{\"cmd\":\"echo $i\"}" >/dev/null
done
append_ms="$(elapsed_ms "$start")"

start="$(start_ns)"
"$BIN" validate --file "$LOG" >/dev/null
validate_ms="$(elapsed_ms "$start")"

start="$(start_ns)"
"$BIN" summarise --file "$LOG" >/dev/null
summary_ms="$(elapsed_ms "$start")"

bytes="$(wc -c < "$LOG")"
lines="$(wc -l < "$LOG")"
append_lines="$(wc -l < "$APPEND_LOG")"

cat <<REPORT
runtrail performance smoke
- events: $lines
- bytes: $bytes
- fixture_generate_ms: $gen_ms
- append_events: $append_lines
- append_ms: $append_ms
- validate_ms: $validate_ms
- summarise_ms: $summary_ms
- log: $LOG
- append_log: $APPEND_LOG
REPORT

if [ "$lines" -ne "$N" ]; then
  echo "expected $N lines, got $lines" >&2
  exit 1
fi

if [ "$append_lines" -ne "$APPEND_N" ]; then
  echo "expected $APPEND_N appended lines, got $append_lines" >&2
  exit 1
fi

if [ "$validate_ms" -gt 1000 ]; then
  echo "validate exceeded 1000ms target" >&2
  exit 1
fi

if [ "$summary_ms" -gt 1000 ]; then
  echo "summarise exceeded 1000ms target" >&2
  exit 1
fi
