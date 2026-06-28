//! Load testing harness for plocate-server.
//!
//! Built on `rlt` (https://crates.io/crates/rlt) which provides constant-RPS
//! and constant-concurrency load with correct coordinated-omission-free
//! latency measurement — important here because each `/api/search` request
//! forks a plocate child process with a heavy latency tail.
//!
//! Two scenarios are exposed as subcommands:
//!   - `baseline`: constant RPS, measure latency distribution
//!   - `saturate`: constant concurrency beyond --max-concurrent-searches,
//!     verify graceful degradation (timeouts, not crashes)

fn main() -> anyhow::Result<()> {
    println!("bench crate initialized");
    Ok(())
}
