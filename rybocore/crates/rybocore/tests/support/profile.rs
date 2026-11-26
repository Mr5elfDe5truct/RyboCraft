//! Writes a test's profile (`#[path]`-included): with the `profile` feature on and
//! `RYBOCORE_PROFILE` set to a path, the kernel's records go there as JSON Lines
//! (`rybocore::profile`). The library itself never reads the environment or writes files.
//!
//! ```sh
//! RYBOCORE_PROFILE=/tmp/p.jsonl cargo test --release -p rybocore --features profile \
//!     --test <file> <test> -- --exact
//! cargo run -p rybocore --example profile_report -- /tmp/p.jsonl /tmp/p.folded
//! ```
#![allow(clippy::expect_used, dead_code)] // test helper; each test uses a subset

use rybocore::Kernel;

/// Writes `k`'s profile to `$RYBOCORE_PROFILE` (nothing without the feature or the
/// variable).
#[cfg(feature = "profile")]
pub fn write_profile(k: &Kernel) {
    use std::io::Write;
    let Some(path) = std::env::var_os("RYBOCORE_PROFILE") else {
        return;
    };
    let file = std::fs::File::create(&path).expect("RYBOCORE_PROFILE is writable");
    let mut out = std::io::BufWriter::new(file);
    k.take_profile()
        .write_jsonl(&mut out)
        .expect("the profile is written");
    out.flush().expect("the profile is written");
}

/// Writes `k`'s profile (nothing: the `profile` feature is off).
#[cfg(not(feature = "profile"))]
pub fn write_profile(_k: &Kernel) {}
