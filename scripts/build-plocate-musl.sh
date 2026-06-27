#!/usr/bin/env bash
#
# Build fully-static musl binaries of `plocate` and `updatedb` from the vendored
# plocate source, linked against vendored static libzstd and libjemalloc.
#
# Toolchain: zig cc / zig c++ (provides musl + static libc++).
# Allocator: jemalloc interposes malloc/free via --whole-archive, so C++ new/delete
#            (and everything else) goes through jemalloc.
#
# Outputs: dist/musl/plocate, dist/musl/updatedb  (stripped)
#
set -euo pipefail

PLOCATE_VER=1.1.24
ZSTD_VER=1.5.6
JEMALLOC_VER=5.3.0
TARGET=x86_64-linux-musl
HOST_TRIPLE=x86_64-unknown-linux-musl

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
THIRD="$ROOT/third_party"
BUILD="$ROOT/build/musl"
SRC="$BUILD/src"
PREFIX="$BUILD/root"
DIST="$ROOT/dist/musl"

ZIG_CC=(zig cc -target "$TARGET")
ZIG_CXX=(zig c++ -target "$TARGET")

log() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

# --- preflight ---------------------------------------------------------------
command -v zig >/dev/null || die "zig not found in PATH"
for f in \
  "plocate-$PLOCATE_VER.tar.gz" \
  "zstd-$ZSTD_VER.tar.gz" \
  "jemalloc-$JEMALLOC_VER.tar.bz2"; do
  [[ -f "$THIRD/$f" ]] || die "missing vendored source: third_party/$f"
done

rm -rf "$BUILD" "$DIST"
mkdir -p "$SRC" "$PREFIX/lib" "$PREFIX/include" "$DIST"

# --- extract -----------------------------------------------------------------
log "extracting sources"
tar xzf "$THIRD/plocate-$PLOCATE_VER.tar.gz"   -C "$SRC"
tar xzf "$THIRD/zstd-$ZSTD_VER.tar.gz"         -C "$SRC"
tar xjf "$THIRD/jemalloc-$JEMALLOC_VER.tar.bz2" -C "$SRC"

PL="$SRC/plocate-$PLOCATE_VER"
ZS="$SRC/zstd-$ZSTD_VER"
JM="$SRC/jemalloc-$JEMALLOC_VER"

# --- patch: avoid linux/stat.h vs musl sys/stat.h statx redefinition ---------
# io_uring_engine.h pulls <linux/stat.h> (kernel UAPI) which collides with
# musl's <sys/stat.h> over `struct statx`. We only need the type, which musl
# already provides, so swap the include.
sed -i 's|#include <linux/stat.h>|#include <sys/stat.h>|' "$PL/io_uring_engine.h"

# --- stage 1: libzstd.a ------------------------------------------------------
log "building libzstd.a (zstd $ZSTD_VER)"
make -C "$ZS/lib" libzstd.a \
  CC="zig cc -target $TARGET" CFLAGS="-O3 -fPIC -DNDEBUG" \
  -j"$(nproc)" >/dev/null
cp "$ZS/lib/libzstd.a" "$PREFIX/lib/"
cp "$ZS/lib/zstd.h" "$ZS/lib/zstd_errors.h" "$ZS/lib/zdict.h" "$PREFIX/include/"

# --- stage 2: libjemalloc.a --------------------------------------------------
log "building libjemalloc.a (jemalloc $JEMALLOC_VER, no C++ bindings)"
(
  cd "$JM"
  # --disable-cxx: skip the C++ wrapper (needs libstdc++/libc++ throw stubs).
  # No --with-jemalloc-prefix: export default malloc/free/... names for interposition.
  ./configure \
    CC="zig cc -target $TARGET" \
    --host="$HOST_TRIPLE" \
    --prefix="$PREFIX" \
    --disable-shared --enable-static --disable-cxx \
    --disable-debug --disable-stats --disable-prof >/dev/null
  make build_lib_static -j"$(nproc)" >/dev/null
)
cp "$JM/lib/libjemalloc.a" "$PREFIX/lib/"

# --- stage 3: plocate + updatedb --------------------------------------------
log "compiling plocate + updatedb (static musl, jemalloc-interposed)"
COMMON_DEFS=(
  -DWITHOUT_URING -DHAS_ENDIAN_H
  -DGROUPNAME=\"plocate\"
  -DDBFILE=\"/var/lib/plocate/plocate.db\"
  -DUPDATEDB_CONF=\"/etc/updatedb.conf\"
  -DPACKAGE_NAME=\"plocate\"
  -DPACKAGE_VERSION=\"$PLOCATE_VER\"
  -DPACKAGE_BUGREPORT=\"steinar+plocate@gunderson.no\"
)
COMMON_FLAGS=(-target "$TARGET" -static -O3 -std=c++17 -DNDEBUG
  "${COMMON_DEFS[@]}" -I"$PREFIX/include" -I"$PL")
# jemalloc first (--whole-archive) so its malloc/free win over musl's weak defs.
LINK=(-Wl,--whole-archive "$PREFIX/lib/libjemalloc.a" -Wl,--no-whole-archive
  -L"$PREFIX/lib" -lzstd -lpthread)

zig c++ "${COMMON_FLAGS[@]}" \
  "$PL"/plocate.cpp "$PL"/io_uring_engine.cpp "$PL"/turbopfor.cpp \
  "$PL"/parse_trigrams.cpp "$PL"/serializer.cpp "$PL"/access_rx_cache.cpp \
  "$PL"/needle.cpp "$PL"/complete_pread.cpp \
  "${LINK[@]}" -o "$DIST/plocate"

zig c++ "${COMMON_FLAGS[@]}" \
  "$PL"/updatedb.cpp "$PL"/database-builder.cpp "$PL"/conf.cpp \
  "$PL"/lib.cpp "$PL"/bind-mount.cpp "$PL"/complete_pread.cpp \
  "${LINK[@]}" -o "$DIST/updatedb"

strip "$DIST/plocate" "$DIST/updatedb"

log "done → $(file -b "$DIST/plocate" | cut -d, -f1-2) / $(file -b "$DIST/updatedb" | cut -d, -f1-2)"
log "  $DIST/plocate   ($(du -h "$DIST/plocate"   | cut -f1))"
log "  $DIST/updatedb  ($(du -h "$DIST/updatedb"  | cut -f1))"
