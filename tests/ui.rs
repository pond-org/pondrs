//! Compile-time diagnostic tests.
//!
//! These lock down the error messages users see when a pipeline is constructed
//! incorrectly. The `.stderr` snapshots are rustc-version specific, so the suite
//! is gated behind the `ui-tests` feature and excluded from a default
//! `cargo test`:
//!
//! ```sh
//! cargo test --features ui-tests              # check snapshots
//! TRYBUILD=overwrite cargo test --features ui-tests   # regenerate them
//! ```
#![cfg(feature = "ui-tests")]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
    t.pass("tests/ui/pass/*.rs");
}
