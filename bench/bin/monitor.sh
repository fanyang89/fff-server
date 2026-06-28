#!/usr/bin/env bash
#
# Sample plocate-server runtime metrics at a fixed interval, emitting one
# JSON object per line on stdout. Designed to run in the background while
# the bench client (rlt) drives load; pipe to a file for later analysis.
#
# Captures:
#   ts                unix timestamp (seconds, float)
#   rss_bytes         server RSS from /api/stats
#   threads           server thread count from /api/stats
#   plocate_count     number of live plocate child processes (pgrep)
#                     — when this equals --max-concurrent-searches, the
#                       semaphore is saturated and new requests are queueing
#   reindexing        whether a reindex is in progress
#
# Usage:
#   monitor.sh <base-url> [interval-secs] > stats.ndjson &
#   MONITOR_PID=$!
#   ...
#   kill $MONITOR_PID
#
# Defaults to 1s interval. Exits cleanly on SIGTERM/SIGINT.
#
set -euo pipefail

URL="${1:?usage: monitor.sh <base-url> [interval-secs]}"
INTERVAL="${2:-1}"

cleanup() { exit 0; }
trap cleanup TERM INT

# jq isoptional; fall back to python if absent.
HAS_JQ=0
command -v jq >/dev/null 2>&1 && HAS_JQ=1

while true; do
  TS=$(date +%s.%N)
  STATS_JSON=$(curl -fsS --max-time 2 "$URL/api/stats" 2>/dev/null || echo '{}')
  # pgrep -c prints the count on stdout but exits 1 when count is 0; capture
  # the line and default to 0 on empty (no `|| echo 0` — that would double it).
  PLOCATE_N=$(pgrep -c '^plocate$' 2>/dev/null || true)
  PLOCATE_N="${PLOCATE_N:-0}"

  if [ "$HAS_JQ" = 1 ]; then
    echo "$STATS_JSON" | jq -c --arg ts "$TS" --argjson pn "$PLOCATE_N" \
      '{ts: ($ts|tonumber),
        rss_bytes: (.process.rss_bytes // 0),
        threads:   (.process.threads // 0),
        plocate_count: $pn,
        reindexing: (.index.reindexing // false)}' 2>/dev/null || \
      printf '{"ts":%s,"error":"stats_parse_failed"}\n' "$TS"
  else
    python3 -c "
import json, sys
s = json.loads(sys.argv[1] or '{}')
print(json.dumps({
    'ts': float(sys.argv[2]),
    'rss_bytes': s.get('process', {}).get('rss_bytes', 0),
    'threads': s.get('process', {}).get('threads', 0),
    'plocate_count': int(sys.argv[3]),
    'reindexing': s.get('index', {}).get('reindexing', False),
}))
" "$STATS_JSON" "$TS" "$PLOCATE_N"
  fi

  sleep "$INTERVAL"
done
