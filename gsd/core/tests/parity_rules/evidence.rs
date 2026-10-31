//! For each code of the code book: a test that makes it fire and a test that keeps it silent (D-r44-3). Both are
//! `#[test]` fns under `tests/` or in a `src/` test module whose name or body names the code; `parity_rules.rs`
//! checks that they exist. Every code has a rule behind it (K20a built W514 and E515).

pub enum Evidence {
    Tests {
        fires: &'static str,
        silent: &'static str,
    },
}

pub const RULE_EVIDENCE: [(&str, Evidence); 63] = [
    (
        "E101",
        Evidence::Tests {
            fires: "unmapped_requirement_is_e101",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E102",
        Evidence::Tests {
            fires: "plan_cycle_is_e102",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W103",
        Evidence::Tests {
            fires: "six_criteria_is_w103",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W104",
        Evidence::Tests {
            fires: "gap_in_phase_numbers_is_w104",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E105",
        Evidence::Tests {
            fires: "roadmap_errors_report_each_code_once",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E106",
        Evidence::Tests {
            fires: "close_milestone_on_the_twin_is_refused_e106_and_e511",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E107",
        Evidence::Tests {
            fires: "a_resolved_blocker_at_a_local_time_is_e107",
            silent: "an_authored_ts_field_gets_e107_and_a_computed_one_does_not",
        },
    ),
    (
        "E108",
        Evidence::Tests {
            fires: "a_second_project_is_e108",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E201",
        Evidence::Tests {
            fires: "e201_a_context_without_canonical_refs_refuses_the_planning_prompts",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E202",
        Evidence::Tests {
            fires: "e202_failed_records",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "W203",
        Evidence::Tests {
            fires: "w203_failed_records_under_strict",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "W204",
        Evidence::Tests {
            fires: "w204_an_open_research_question_refuses_every_planning_prompt_under_strict",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W205",
        Evidence::Tests {
            fires: "a_suspicious_package_fails_w205",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "E206",
        Evidence::Tests {
            fires: "e206_a_second_research_in_the_phase_refuses_the_planning_prompts",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E301",
        Evidence::Tests {
            fires: "phase_requirement_in_no_plan_is_e301",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E302",
        Evidence::Tests {
            fires: "plan_without_requirements_is_e302",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E303",
        Evidence::Tests {
            fires: "e303_fires_on_an_auto_task_without_files_or_verify",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W304",
        Evidence::Tests {
            fires: "w304_is_four_tasks_and_e322_is_five_never_both",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W305",
        Evidence::Tests {
            fires: "same_wave_overlap_is_w305",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W306",
        Evidence::Tests {
            fires: "task_file_outside_plan_is_w306",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E307",
        Evidence::Tests {
            fires: "e307_names_the_available_options",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W308",
        Evidence::Tests {
            fires: "w308_is_an_error_under_gsd_strict",
            silent: "w308_is_silent_when_settled_or_after_a_decision_checkpoint",
        },
    ),
    (
        "W309",
        Evidence::Tests {
            fires: "w309_needs_both_a_truth_and_an_artifact",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E310",
        Evidence::Tests {
            fires: "an_uncited_context_decision_is_e310",
            silent: "context_coverage_gate_off_silences_e310",
        },
    ),
    (
        "W311",
        Evidence::Tests {
            fires: "a_plan_without_a_verdict_reports_w311_to_w316",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "E312",
        Evidence::Tests {
            fires: "an_action_for_now_static_is_e312",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W313",
        Evidence::Tests {
            fires: "a_passing_verdict_silences_each_judged_code",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "E314",
        Evidence::Tests {
            fires: "a_task_without_verify_or_wave0_is_e314",
            silent: "nyquist_validation_off_silences_e314_and_w510",
        },
    ),
    (
        "W315",
        Evidence::Tests {
            fires: "a_passing_verdict_silences_each_judged_code",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "W316",
        Evidence::Tests {
            fires: "a_plan_without_a_verdict_reports_w311_to_w316",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "W317",
        Evidence::Tests {
            fires: "an_uncovered_phase_requirement_is_w317",
            silent: "post_planning_gaps_off_silences_w317",
        },
    ),
    (
        "W318",
        Evidence::Tests {
            fires: "a_stale_plan_check_is_w318",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E319",
        Evidence::Tests {
            fires: "each_plan_shape_code_fires_on_its_fixture",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E320",
        Evidence::Tests {
            fires: "a_missing_directory_is_e320",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "I321",
        Evidence::Tests {
            fires: "a_justified_coupling_is_i321",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E322",
        Evidence::Tests {
            fires: "w304_is_four_tasks_and_e322_is_five_never_both",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W323",
        Evidence::Tests {
            fires: "a_plan_without_its_phase_is_w323",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W324",
        Evidence::Tests {
            fires: "a_dynamic_path_is_w324",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E507",
        Evidence::Tests {
            fires: "automated_without_fails_when_is_e507",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E401",
        Evidence::Tests {
            fires: "e401_names_the_missing_and_the_extra_ids",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E402",
        Evidence::Tests {
            fires: "each_execute_code_fires_on_its_fixture",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W403",
        Evidence::Tests {
            fires: "w403_needs_a_red_commit_first_and_tdd_mode_on",
            silent: "gate_cutover_tdd_verdicts_reproduce",
        },
    ),
    (
        "W404",
        Evidence::Tests {
            fires: "a_tdd_plan_without_review_is_w404",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E405",
        Evidence::Tests {
            fires: "e405_names_the_orm_and_its_push_command",
            silent: "e405_is_clean_with_push_evidence_with_the_gate_off_and_before_execution",
        },
    ),
    (
        "W406",
        Evidence::Tests {
            fires: "w406_a_drifted_context_refuses_the_planning_prompts_when_the_action_is_block",
            silent: "gate_cutover_context_drift_verdicts_reproduce",
        },
    ),
    (
        "W407",
        Evidence::Tests {
            fires: "task_without_commit_is_w407_only_when_task_commits_is_on",
            silent: "task_without_commit_is_w407_only_when_task_commits_is_on",
        },
    ),
    (
        "W408",
        Evidence::Tests {
            fires: "a_failing_check_is_w408",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E501",
        Evidence::Tests {
            fires: "a_verified_truth_records",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
    (
        "E502",
        Evidence::Tests {
            fires: "a_verified_verdict_the_tests_contradict_is_refused_naming_e502",
            silent: "e502_is_silent_without_a_report",
        },
    ),
    (
        "E503",
        Evidence::Tests {
            fires: "an_override_without_by_is_e503",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W504",
        Evidence::Tests {
            fires: "four_overrides_is_w504",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E505",
        Evidence::Tests {
            fires: "todo_in_a_phase_file_is_e505",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W506",
        Evidence::Tests {
            fires: "a_decision_nothing_shipped_is_w506_on_the_decision",
            silent: "w506_a_summary_or_a_commit_that_names_the_decision_covers_it",
        },
    ),
    (
        "E509",
        Evidence::Tests {
            fires: "a_skip_without_reason_is_e509",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W510",
        Evidence::Tests {
            fires: "a_phase_with_validation_architecture_and_no_validation_is_w510",
            silent: "nyquist_validation_off_silences_e314_and_w510",
        },
    ),
    (
        "E511",
        Evidence::Tests {
            fires: "close_milestone_on_the_twin_is_refused_e106_and_e511",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E512",
        Evidence::Tests {
            fires: "ship_with_an_open_window_is_refused_e512",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "E513",
        Evidence::Tests {
            fires: "waived_and_fixed_is_e513",
            silent: "the_clean_twin_is_silent_for_the_unjudged_codes",
        },
    ),
    (
        "W514",
        Evidence::Tests {
            fires: "a_refactor_deviation_window_with_enforcement_off_is_w514",
            silent: "windows_enforce_on_silences_w514",
        },
    ),
    (
        "E515",
        Evidence::Tests {
            fires: "e515_refuses_close_milestone_under_plus_until_the_requirement_traces",
            silent: "e515_is_silent_by_default",
        },
    ),
    (
        "E516",
        Evidence::Tests {
            fires: "e516_a_stale_verdict_refuses_complete_milestone_and_ship_under_plus",
            silent: "e516_a_stale_verdict_refuses_complete_milestone_and_ship_under_plus",
        },
    ),
    (
        "E517",
        Evidence::Tests {
            fires: "e517_fires_on_overlapping_active_plans_under_plus",
            silent: "e517_is_silent_by_default",
        },
    ),
    (
        "E518",
        Evidence::Tests {
            fires: "an_audit_without_a_verdict_reports_e518",
            silent: "a_passing_verdict_silences_each_judged_code",
        },
    ),
];
