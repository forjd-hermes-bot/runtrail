#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo build --release >/dev/null
BIN="$ROOT/target/release/runtrail"
TMP="${TMPDIR:-/tmp}/runtrail-perf-$$"
mkdir -p "$TMP"
LOG="$TMP/events.jsonl"
N="${1:-10000}"

start_ns() { date +%s%N; }
elapsed_ms() {
  local start="$1"
  local end
  end="$(start_ns)"
  echo $(( (end - start) / 1000000 ))
}

start="$(start_ns)"
for i in $(seq 1 "$N"); do
  "$BIN" log --file "$LOG" --event command.run --attr exit_code=0 --body "{\"cmd\":\"echo $i\"}" >/dev/null
done
gen_ms="$(elapsed_ms "$start")"

start="$(start_ns)"
"$BIN" validate --file "$LOG" >/dev/null
validate_ms="$(elapsed_ms "$start")"

start="$(start_ns)"
"$BIN" summarise --file "$LOG" >/dev/null
summary_ms="$(elapsed_ms "$start")"

bytes="$(wc -c < "$LOG")"
lines="$(wc -l < "$LOG")"

cat <<REPORT
runtrail performance smoke
- events: $lines
- bytes: $bytes
- generate_ms: $gen_ms
- validate_ms: $validate_ms
- summarise_ms: $summary_ms
- log: $LOG
REPORT

if [ "$lines" -ne "$N" ]; then
  echo "expected $N lines, got $lines" >&2
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
