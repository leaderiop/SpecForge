//! MCP validate judges history per request (ADR 0287).

use specforge_test_macros::test as specforge_test;

#[specforge_test(
    behavior = "read_git_baseline",
    verify = "validate since judges per request"
)]
#[ignore = "K4 T11"]
fn validate_since_judges_per_request() {
    // validate with since main reports E084; without since it does not; the served session keeps HEAD
    unimplemented!(
        "K4 T11: validate with since main reports E084; without since it does not; the served session keeps HEAD"
    );
}
