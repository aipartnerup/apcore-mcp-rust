//! Cross-language conformance: middleware Config Bus loading.
//!
//! Drives the Rust builder from the shared fixture at
//! `apcore-mcp/conformance/fixtures/middleware_config.json`. The Python and
//! TypeScript bridges run the same fixture through their own builders; all
//! three implementations must agree on the resulting middleware names and on
//! which inputs are rejected.

use std::path::{Path, PathBuf};

use apcore_mcp::middleware_builder::build_middleware_from_config;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
struct Fixture {
    test_cases: Vec<SuccessCase>,
    error_cases: Vec<ErrorCase>,
}

#[derive(Deserialize)]
struct SuccessCase {
    id: String,
    input_entries: Value,
    expected_middleware_names: Vec<String>,
}

#[derive(Deserialize)]
struct ErrorCase {
    id: String,
    input_entries: Value,
    expected_error_substring: String,
}

fn fixture_path() -> Option<PathBuf> {
    // Walk up from the crate root looking for the sibling `apcore-mcp` repo.
    let mut dir: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    for _ in 0..4 {
        let candidate = dir
            .join("apcore-mcp")
            .join("conformance")
            .join("fixtures")
            .join("middleware_config.json");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn load_fixture() -> Option<Fixture> {
    let path: &Path = &fixture_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

#[test]
fn conformance_success_cases() {
    let Some(fixture) = load_fixture() else {
        eprintln!("skipping: conformance fixture not found — run tests from the monorepo layout");
        return;
    };

    for case in &fixture.test_cases {
        let result = build_middleware_from_config(Some(&case.input_entries));
        let mws = match result {
            Ok(mws) => mws,
            Err(e) => panic!("case {}: unexpected error: {e}", case.id),
        };
        let names: Vec<&str> = mws.iter().map(|mw| mw.name()).collect();
        assert_eq!(
            names, case.expected_middleware_names,
            "case {}: names mismatch",
            case.id
        );
    }
}

#[test]
fn conformance_error_cases() {
    let Some(fixture) = load_fixture() else {
        eprintln!("skipping: conformance fixture not found — run tests from the monorepo layout");
        return;
    };

    for case in &fixture.error_cases {
        let result = build_middleware_from_config(Some(&case.input_entries));
        let err = match result {
            Err(e) => format!("{e}"),
            Ok(_) => panic!(
                "case {}: expected error containing {:?} but build succeeded",
                case.id, case.expected_error_substring
            ),
        };
        assert!(
            err.contains(&case.expected_error_substring),
            "case {}: error message {:?} missing substring {:?}",
            case.id,
            err,
            case.expected_error_substring,
        );
    }
}
