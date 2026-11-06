//! Compile-time diagnostic tests.
//!
//! These lock down the error messages users see when a pipeline is constructed
//! incorrectly. The `.stderr` snapshots are rustc-version specific, so the suite
//! is gated behind the `ui-tests` feature and excluded from a default
//! `cargo test`. They also depend on the feature set, since rustc lists
//! feature-gated impls in them, so always run with exactly these features:
//!
//! ```sh
//! cargo test --no-default-features --features std,ui-tests --test ui   # check snapshots
//! TRYBUILD=overwrite cargo test --no-default-features --features std,ui-tests --test ui   # regenerate them
//! ```
#![cfg(feature = "ui-tests")]

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
    t.pass("tests/ui/pass/*.rs");
}
