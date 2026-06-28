#!/usr/bin/env bash
#
# Generate a synthetic artifact-tree index for benchmarking plocate-server.
#
# Produces three things under bench/data/:
#   tree/               empty placeholder files shaped like a real artifact
#                       repository (RPMs, ISOs, source tarballs, debuginfo).
#                       Updatedb walks it; plocate indexes paths only, so the
#                       files are kept empty.
#   files.db            the plocate trigram index for tree/.
#   queries.txt         a curated query corpus for the bench harness — one
#                       query per line; # comments are ignored.
#
# Sizes (positional arg 1, default 10k):
#   10k     ~3 MB tree, ~1 MB db     quick smoke
#   100k    ~30 MB tree, ~10 MB db   realistic single-host repo
#   1m      ~300 MB tree, ~100 MB db stress / saturation finding
#
# Re-runnable: wipes tree/ and files.db on each invocation.
#
# Requires: updatedb, plocate (provided by the plocate package), coreutils.
#
set -euo pipefail

SIZE="${1:-10k}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DATA="$ROOT/data"
TREE="$DATA/tree"
DB="$DATA/files.db"
QUERIES="$DATA/queries.txt"
PATHS_TMP="$DATA/.paths.tmp"

case "$SIZE" in
  10k) COUNT=10000 ;;
  100k) COUNT=100000 ;;
  1m) COUNT=1000000 ;;
  *) echo "usage: $0 [10k|100k|1m]" >&2; exit 1 ;;
esac

require() { command -v "$1" >/dev/null 2>&1 || { echo "missing: $1" >&2; exit 1; }; }
require updatedb
require plocate

log() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }

# --- path-shape tables -------------------------------------------------------
# Mimic a Fedora/CentOS/Debian/Ubuntu-style repository so plocate's index
# characteristics (path lengths, trigram diversity, basename repetition) are
# realistic. The actual bytes are irrelevant — plocate stores paths only.

DISTROS=(fedora centos-stream debian ubuntu)
FEDORA_VERSIONS=(38 39 40 41)
CENTOS_VERSIONS=(8 9)
DEBIAN_VERSIONS=(bookworm trixie)
UBUNTU_VERSIONS=(jammy noble)
ARCHES=(x86_64 aarch64)
PACKAGE_PREFIXES=(
  kernel-core kernel-modules kernel-devel kernel-headers
  glibc glibc-devel glibc-common
  openssl openssl-libs openssl-devel
  systemd systemd-libs systemd-devel
  curl libcurl libcurl-devel
  bash coreutils findutils
  python3 python3-libs python3-devel
  rust rust-std cargo
  gcc gcc-c++ libgcc libstdc++
  zlib zlib-devel
  libxml2 libxml2-devel
  sqlite sqlite-devel
  postgresql postgresql-libs
  redis valkey
  nginx httpd
)
ISO_NAMES=(Server Workstation Everything Netinst Minimal Live)

# rand <bound>: uniform [0, bound) from /dev/urandom.
rand() {
  local r
  r=$(od -An -N4 -tu4 < /dev/urandom | tr -d ' ')
  echo $(( r % $1 ))
}
# pick <inline-var-name>: echo a random word from the space-separated string
# stored in the named environment variable. We pass by var name (not value)
# so this works the same in the parent shell and in xargs-spawned children
# (bash exports plain-string env vars but NOT arrays).
pick() {
  local name="$1"
  local words
  read -ra words <<< "${!name}"
  echo "${words[$(rand ${#words[@]})]}"
}

# gen_one: emit one path (relative, no TREE/ prefix) on stdout.
# Distribution: 80% RPM/deb, 10% ISO, 5% source tarball, 5% debuginfo.
gen_one() {
  local roll distro ver rel arch pkg maj min pat dist iso
  roll=$(rand 100)
  distro=$(pick INLINE_DISTROS)
  case "$distro" in
    fedora)        ver=$(pick INLINE_FEDORA); rel=$(rand 30);  dist="fc$ver" ;;
    centos-stream) ver=$(pick INLINE_CENTOS);  rel=$(rand 200); dist="el$ver" ;;
    debian)        ver=$(pick INLINE_DEBIAN);  rel=$(rand 10);  dist="$ver" ;;
    ubuntu)        ver=$(pick INLINE_UBUNTU);  rel=$(rand 30);  dist="$ver" ;;
  esac
  arch=$(pick INLINE_ARCHES)
  pkg=$(pick INLINE_PKGS)
  maj=$(rand 20); min=$(rand 50); pat=$(rand 10)
  if [ "$roll" -lt 80 ]; then
    if [ "$distro" = "debian" ] || [ "$distro" = "ubuntu" ]; then
      printf '%s/Packages/%s/%s/%s_%s.%s_%s.deb\n' \
        "$distro" "$ver" "$arch" "$pkg" "${maj}.${min}.${pat}-${rel}" "$dist" "$arch"
    else
      printf '%s/Packages/%s/%s/%s-%s.%s-%s.%s.rpm\n' \
        "$distro" "$ver" "$arch" "$pkg" "${maj}.${min}.${pat}" "$rel" "$dist" "$arch"
    fi
  elif [ "$roll" -lt 90 ]; then
    iso=$(pick INLINE_ISOS)
    printf '%s/ISOs/%s/%s-%s-%s.%s.iso\n' \
      "$distro" "$ver" "$(printf '%s' "$distro" | cut -c1-6)" "$iso" "$ver" "$arch"
  elif [ "$roll" -lt 95 ]; then
    printf 'sources/%s-%s.%s.tar.xz\n' "$pkg" "${maj}.${min}.${pat}" "$(rand 100)"
  else
    printf '%s/debug/%s-debuginfo-%s.%s.%s.x86_64.rpm\n' \
      "$distro" "$pkg" "${maj}.${min}.${pat}" "$(rand 30)" "$(rand 5)"
  fi
}

# Export the helpers so they can run in subprocesses (note: bash does NOT
# export arrays — DISTROS etc. are re-declared in each subprocess via the
# INLINE_VARS trick below).
export -f gen_one pick rand

# Bash arrays are not exported across exec. Inline the table values into the
# subprocess environment as plain strings; pick() parses them on the fly.
INLINE_DISTROS="${DISTROS[*]}"
INLINE_FEDORA="${FEDORA_VERSIONS[*]}"
INLINE_CENTOS="${CENTOS_VERSIONS[*]}"
INLINE_DEBIAN="${DEBIAN_VERSIONS[*]}"
INLINE_UBUNTU="${UBUNTU_VERSIONS[*]}"
INLINE_ARCHES="${ARCHES[*]}"
INLINE_PKGS="${PACKAGE_PREFIXES[*]}"
INLINE_ISOS="${ISO_NAMES[*]}"
export INLINE_DISTROS INLINE_FEDORA INLINE_CENTOS INLINE_DEBIAN INLINE_UBUNTU \
       INLINE_ARCHES INLINE_PKGS INLINE_ISOS

log "cleaning $TREE / $DB / $QUERIES"
rm -rf "$TREE" "$DB" "$QUERIES" "$PATHS_TMP"
mkdir -p "$TREE"

log "generating $COUNT candidate paths (parallel, $(nproc) workers)"
# -I{} implies one input item per invocation; -P gives parallelism.
seq 1 "$COUNT" | xargs -P"$(nproc)" -I{} bash -c 'gen_one' \
  | awk 'NF' | sort -u > "$PATHS_TMP"
UNIQUE=$(wc -l < "$PATHS_TMP")
log "unique paths: $UNIQUE"

log "creating directories"
awk -F/ '{NF--; print}' OFS=/ "$PATHS_TMP" | sort -u | sed "s|^|$TREE/|" \
  | xargs -d '\n' mkdir -p

log "touching placeholder files"
sed "s|^|$TREE/|" "$PATHS_TMP" | xargs -d '\n' touch
rm -f "$PATHS_TMP"

# --- build the plocate index ------------------------------------------------
log "running updatedb"
# plocate's updatedb options: --require-visibility no disables permission
# checks (we want every file indexed regardless of who owns it); empty prune
# options defeat any system-wide /etc/updatedb.conf pruning that would skew
# the synthetic tree.
updatedb \
  -U "$TREE" \
  -o "$DB" \
  --require-visibility no \
  --prune-bind-mounts no \
  --prunefs "" \
  --prunenames "" \
  --prunepaths ""

# --- queries corpus ----------------------------------------------------------
log "writing $QUERIES"
{
  echo "# Curated query corpus for plocate-server bench."
  echo "# One query per line; lines starting with # are ignored by the harness."
  echo "# Sections group queries by intended --mode (substring / glob / fuzzy)."
  echo
  echo "# --- substrings (package basenames; use --mode substring) ---"
  printf '%s\n' kernel-core glibc openssl systemd libcurl python3 rust-std \
                postgresql valkey nginx httpd debuginfo sources
  echo
  echo "# --- extensions (substring matches all .rpm / .iso / .deb files) ---"
  printf '%s\n' .rpm .iso .deb .tar.xz
  echo
  echo "# --- version / arch fragments ---"
  printf '%s\n' fc40 el9 bookworm noble jammy aarch64
  echo
  echo "# --- globs (use --mode glob) ---"
  printf '%s\n' 'kernel-*.rpm' '*-devel-*.rpm' 'glibc-[0-9]*' '*.iso' '*-debuginfo-*'
  echo
  echo "# --- fuzzy multi-keyword (use --mode fuzzy) ---"
  printf '%s\n' 'kernel x86' 'openssl devel' 'python3 libs' 'glibc common' 'rust std'
} > "$QUERIES"

log "done"
log "  tree:    $(du -sh "$TREE" | cut -f1) ($UNIQUE files)"
log "  index:   $(du -sh "$DB" | cut -f1)"
log "  queries: $(grep -cv '^[[:space:]]*\(#\|$\)' "$QUERIES") usable lines"
log ""
log "next: task bench-serve"
log "     task bench-baseline -- --rate 100 --duration 5m"
log "     task bench-saturate -- --concurrency 64 --duration 2m"
