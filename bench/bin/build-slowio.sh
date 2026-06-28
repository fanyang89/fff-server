#!/usr/bin/env bash
#
# Compile the LD_PRELOAD stat-latency shim used by bench-serve-hdd.
# Idempotent: skips the build if slowio.so is newer than slowio.c.
#
set -euo pipefail

SRC="$(cd "$(dirname "$0")" && pwd)/slowio.c"
OUT="$(cd "$(dirname "$0")" && pwd)/slowio.so"

if [ -f "$OUT" ] && [ "$SRC" -ot "$OUT" ]; then
    exit 0
fi

if ! command -v gcc >/dev/null 2>&1; then
    echo "build-slowio: gcc not found (install gcc / build-essential)" >&2
    exit 1
fi

gcc -O2 -fPIC -shared -o "$OUT" "$SRC" -ldl
echo "built $OUT"
