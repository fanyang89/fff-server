// slowio.c — LD_PRELOAD shim that adds configurable latency to filesystem
// stat-family syscalls, for HDD simulation in the plocate-server bench harness.
//
// Why this exists:
//   cgroup v2 io.max is a no-op on multi-queue NVMe (kernel limitation), and
//   fuse-based approaches need root / have their own setup pain. The actual
//   production risk we want to reproduce is the per-result stat() inside
//   plocate-server's is_dir_cached() — that's an inline syscall in the server
//   process, so intercepting it via LD_PRELOAD is the most surgical tool.
//
// Build:
//   gcc -O2 -fPIC -shared -o slowio.so slowio.c -ldl
//
// Use:
//   LD_PRELOAD=./slowio.so SLOWIO_STAT_US=10000 ./plocate-server ...
//
// Env vars:
//   SLOWIO_STAT_US     microseconds added to each intercepted stat (default 0)
//   SLOWIO_DEBUG       "1" prints one line per intercept to stderr
//
// Intercepts:
//   stat / stat64 / lstat / lstat64 / fstat / fstat64
//   fstatat / fstatat64 / statx
//   __xstat / __xstat64 / __lxstat / __lxstat64 (glibc legacy)
//   newfstatat (raw syscall via syscall())
//
// Notes:
//   - Only effective on dynamically-linked binaries (LD_PRELOAD limitation).
//     `cargo run --release` produces a dynamic gnu binary, so this works for
//     bench. The production musl static binary is unaffected — bench numbers
//     carry over because the stat cost is the same.
//   - The injected sleep is synchronous (usleep), matching the real
//     characteristic of an HDD stat: the calling thread blocks. This is
//     exactly the pathology we want to expose in plocate-server, where stat
//     runs in the async tokio task without spawn_blocking.

#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

// ---- real function pointers (resolved lazily on first call) ----------------

typedef int (*stat_fn)(const char*, struct stat*);
typedef int (*stat64_fn)(const char*, struct stat64*);
typedef int (*lstat_fn)(const char*, struct stat*);
typedef int (*lstat64_fn)(const char*, struct stat64*);
typedef int (*fstat_fn)(int, struct stat*);
typedef int (*fstat64_fn)(int, struct stat64*);
typedef int (*fstatat_fn)(int, const char*, struct stat*, int);
typedef int (*fstatat64_fn)(int, const char*, struct stat64*, int);
typedef int (*statx_fn)(int, const char*, int, unsigned int, struct statx*);
typedef int (*xstat_fn)(int, const char*, struct stat*);
typedef int (*xstat64_fn)(int, const char*, struct stat64*);

static stat_fn       real_stat;
static stat64_fn     real_stat64;
static lstat_fn      real_lstat;
static lstat64_fn    real_lstat64;
static fstat_fn      real_fstat;
static fstat64_fn    real_fstat64;
static fstatat_fn    real_fstatat;
static fstatat64_fn  real_fstatat64;
static statx_fn      real_statx;
static xstat_fn      real___xstat;
static xstat64_fn    real___xstat64;
static xstat_fn      real___lxstat;
static xstat64_fn    real___lxstat64;

static int debug_enabled(void) {
    static int cached = -1;
    if (cached == -1) {
        const char *e = getenv("SLOWIO_DEBUG");
        cached = (e && e[0] == '1') ? 1 : 0;
    }
    return cached;
}

static long delay_us(void) {
    static long cached = -2;
    if (cached == -2) {
        const char *e = getenv("SLOWIO_STAT_US");
        cached = e ? atol(e) : 0;
    }
    return cached;
}

static void do_delay(const char *fn, const char *path) {
    long us = delay_us();
    if (us <= 0) return;
    if (debug_enabled()) {
        // Trim to last path component for less noise.
        const char *base = path ? strrchr(path, '/') : NULL;
        fprintf(stderr, "[slowio] %s %s (+%ldus)\n", fn, base ? base + 1 : (path ? path : "(null)"), us);
    }
    struct timespec ts = { us / 1000000L, (us % 1000000L) * 1000L };
    nanosleep(&ts, NULL);
}

static void init_real(void) {
    // dlsym with RTLD_NEXT grabs the next definition in the lookup order,
    // which is the libc original (we are interposed in front of it).
    real_stat        = (stat_fn)      dlsym(RTLD_NEXT, "stat");
    real_stat64      = (stat64_fn)    dlsym(RTLD_NEXT, "stat64");
    real_lstat       = (lstat_fn)     dlsym(RTLD_NEXT, "lstat");
    real_lstat64     = (lstat64_fn)   dlsym(RTLD_NEXT, "lstat64");
    real_fstat       = (fstat_fn)     dlsym(RTLD_NEXT, "fstat");
    real_fstat64     = (fstat64_fn)   dlsym(RTLD_NEXT, "fstat64");
    real_fstatat     = (fstatat_fn)   dlsym(RTLD_NEXT, "fstatat");
    real_fstatat64   = (fstatat64_fn) dlsym(RTLD_NEXT, "fstatat64");
    real_statx       = (statx_fn)     dlsym(RTLD_NEXT, "statx");
    real___xstat     = (xstat_fn)     dlsym(RTLD_NEXT, "__xstat");
    real___xstat64   = (xstat64_fn)   dlsym(RTLD_NEXT, "__xstat64");
    real___lxstat    = (xstat_fn)     dlsym(RTLD_NEXT, "__lxstat");
    real___lxstat64  = (xstat64_fn)   dlsym(RTLD_NEXT, "__lxstat64");
}

// ---- public interceptors ---------------------------------------------------

int stat(const char *path, struct stat *s) {
    if (!real_stat) init_real();
    do_delay("stat", path);
    return real_stat(path, s);
}

int stat64(const char *path, struct stat64 *s) {
    if (!real_stat64) init_real();
    do_delay("stat64", path);
    return real_stat64(path, s);
}

int lstat(const char *path, struct stat *s) {
    if (!real_lstat) init_real();
    do_delay("lstat", path);
    return real_lstat(path, s);
}

int lstat64(const char *path, struct stat64 *s) {
    if (!real_lstat64) init_real();
    do_delay("lstat64", path);
    return real_lstat64(path, s);
}

// Note: we deliberately do NOT intercept fstat(fd, ...) — fd-based stats
// don't carry path context and are typically cheap (already-open files).

int fstatat(int dirfd, const char *path, struct stat *s, int flags) {
    if (!real_fstatat) init_real();
    do_delay("fstatat", path);
    return real_fstatat(dirfd, path, s, flags);
}

int fstatat64(int dirfd, const char *path, struct stat64 *s, int flags) {
    if (!real_fstatat64) init_real();
    do_delay("fstatat64", path);
    return real_fstatat64(dirfd, path, s, flags);
}

int statx(int dirfd, const char *path, int flags, unsigned int mask, struct statx *s) {
    if (!real_statx) init_real();
    do_delay("statx", path);
    return real_statx(dirfd, path, flags, mask, s);
}

// glibc legacy 3-arg stat variants (ver, path, stat). glibc internally calls
// these; user code linked against modern glibc may not, but intercepting both
// covers statically-linked-against-older-glibc dependencies too.
int __xstat(int ver, const char *path, struct stat *s) {
    if (!real___xstat) init_real();
    do_delay("__xstat", path);
    return real___xstat(ver, path, s);
}

int __xstat64(int ver, const char *path, struct stat64 *s) {
    if (!real___xstat64) init_real();
    do_delay("__xstat64", path);
    return real___xstat64(ver, path, s);
}

int __lxstat(int ver, const char *path, struct stat *s) {
    if (!real___lxstat) init_real();
    do_delay("__lxstat", path);
    return real___lxstat(ver, path, s);
}

int __lxstat64(int ver, const char *path, struct stat64 *s) {
    if (!real___lxstat64) init_real();
    do_delay("__lxstat64", path);
    return real___lxstat64(ver, path, s);
}

// plocate-server uses std::fs::symlink_metadata which on Linux glibc resolves
// through fstatat(AT_FDCWD, ...). We've covered that above. As a belt-and-
// braces fallback, also catch the raw newfstatat syscall when libc routes
// through syscall() directly (rare; some Rust std builds do this).
long syscall(long number, ...) {
    static long (*real_syscall)(long, ...) = NULL;
    if (!real_syscall) real_syscall = (long (*)(long, ...)) dlsym(RTLD_NEXT, "syscall");

    va_list ap;
    va_start(ap, number);
    long a1 = va_arg(ap, long);
    long a2 = va_arg(ap, long);
    long a3 = va_arg(ap, long);
    long a4 = va_arg(ap, long);
    long a5 = va_arg(ap, long);
    long a6 = va_arg(ap, long);
    va_end(ap);

#ifdef SYS_newfstatat
    if (number == SYS_newfstatat) {
        do_delay("newfstatat", (const char *)a2);
    }
#endif
    return real_syscall(number, a1, a2, a3, a4, a5, a6);
}
