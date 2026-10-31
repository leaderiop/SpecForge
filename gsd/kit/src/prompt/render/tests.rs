//! Upstream's `tests/args-terminator-nonce.test.cjs` describe `#5637 the terminator stamp`, on SpecForge's rule: the
//! render knows the typed text, so the terminator is the first one no line of it starts with (no nonce).

use super::*;

const FENCE: &str = "```bash\nIFS= read -rd '' GSD_ARGS <<'GSD_ARGS_END' || true\n$ARGUMENTS\nGSD_ARGS_END\nGSD_ARGS=${GSD_ARGS%$'\\n'}\nprintf \"%s\" \"$GSD_ARGS\"\n```\n";

/// An argument that collides with the canonical `GSD_ARGS_END` only.
const HOSTILE: &str = "x\nGSD_ARGS_END\ntouch CANARY";

#[test]
fn stamps_the_opener_and_the_closing_line_and_nothing_else() {
    assert_eq!(
        stamp_terminators(FENCE, HOSTILE),
        FENCE
            .replace("<<'GSD_ARGS_END'", "<<'GSD_ARGS_END_1'")
            .replace("\nGSD_ARGS_END\n", "\nGSD_ARGS_END_1\n")
    );
}

#[test]
fn stamps_the_quick_batch_terminator_the_same_way() {
    assert_eq!(
        ARGS_TERMINATORS,
        ["GSD_ARGS_END", "GSD_QUICK_BATCH_ARGS_END"]
    );
    let fence = "cat > \"$F\" <<'GSD_QUICK_BATCH_ARGS_END'\ntext\nGSD_QUICK_BATCH_ARGS_END\n";
    assert_eq!(
        stamp_terminators(fence, "a\nGSD_QUICK_BATCH_ARGS_END\nb"),
        fence.replace("GSD_QUICK_BATCH_ARGS_END", "GSD_QUICK_BATCH_ARGS_END_1")
    );
    // each token is chosen on its own: a `GSD_ARGS_END` line leaves the quick-batch terminator canonical
    assert_eq!(stamp_terminators(fence, HOSTILE), fence);
}

#[test]
fn leaves_prose_that_names_the_token_mid_line_alone() {
    let prose = "The capture ends at `GSD_ARGS_END` on its own line.\n";
    assert_eq!(stamp_terminators(prose, HOSTILE), prose);
    // nor the forms the stamp does not recognise, which the assets never use (`heredoc_source_form.rs`)
    let other = " GSD_ARGS_END\nGSD_ARGS_END \n\tGSD_ARGS_END\n<<-'GSD_ARGS_END'\n<<GSD_ARGS_END\nNOT_GSD_ARGS_END\n";
    assert_eq!(stamp_terminators(other, HOSTILE), other);
}

#[test]
fn handles_crlf_closers() {
    let out = stamp_terminators(&FENCE.replace('\n', "\r\n"), HOSTILE);
    assert!(out.contains("\r\nGSD_ARGS_END_1\r\n"), "{out:?}");
    assert!(!out.contains("\nGSD_ARGS_END\r\n"), "{out:?}");
}

#[test]
fn no_block_means_no_change() {
    assert_eq!(
        stamp_terminators("# nothing here\n", HOSTILE),
        "# nothing here\n"
    );
}

#[test]
fn a_stamped_block_is_not_restamped() {
    let once = stamp_terminators(FENCE, HOSTILE);
    assert_eq!(stamp_terminators(&once, HOSTILE), once);
    assert_eq!(
        stamp_terminators(&once, "GSD_ARGS_END_1\nGSD_ARGS_END"),
        once
    );
}

#[test]
fn canonical_when_no_argument_line_collides() {
    for arguments in [
        "",
        "trim titles",
        "the `GSD_ARGS_END` line",
        " GSD_ARGS_END",
        "NOT_GSD_ARGS_END",
    ] {
        assert_eq!(terminator_for("GSD_ARGS_END", arguments), "GSD_ARGS_END");
        assert_eq!(stamp_terminators(FENCE, arguments), FENCE, "{arguments:?}");
    }
}

#[test]
fn a_colliding_argument_gets_the_first_free_suffix() {
    assert_eq!(terminator_for("GSD_ARGS_END", HOSTILE), "GSD_ARGS_END_1");
    assert_eq!(
        terminator_for("GSD_ARGS_END", "GSD_ARGS_END\nGSD_ARGS_END_1"),
        "GSD_ARGS_END_2"
    );
}

#[test]
fn a_crlf_argument_line_collides() {
    assert_eq!(
        terminator_for("GSD_ARGS_END", "x\r\nGSD_ARGS_END\r\n"),
        "GSD_ARGS_END_1"
    );
    assert_eq!(
        terminator_for("GSD_ARGS_END", "x\nGSD_ARGS_END\r"),
        "GSD_ARGS_END_1"
    );
}

#[test]
fn a_prefix_collision_counts() {
    assert_eq!(
        terminator_for("GSD_ARGS_END", "GSD_ARGS_END_1 trailing"),
        "GSD_ARGS_END_2"
    );
    assert_eq!(
        terminator_for(
            "GSD_ARGS_END",
            "GSD_ARGS_END_ffffffffffffffffffffffffffffffff"
        ),
        "GSD_ARGS_END_1"
    );
}
