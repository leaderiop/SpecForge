//! What a program a guest runs starts with (ADR 0335 D4): nothing of the host's environment but what a program needs to
//! work and what its capability declares, so what a person approved is what runs, and nothing else leaves. The real
//! runner, the real inherited environment.
#![cfg(unix)]

use std::sync::Arc;

use specforge_effects::testing::{self, MemoryConsent};
use specforge_effects::{
    Approval, Consent, EffectJournal, Effects, EffectsCeiling, EnvVar, Grant, Ports,
    ProcessRequest, Provenance, Roots,
};
use specforge_extension_sdk::prelude::*;
use specforge_protocol_types::{ExtensionDeclaration, HostError, Operation};
use specforge_wasm::{Limits, PortCall, PortSession};

const EXT: &str = "@t/env";
const VALUE: &str = "must-not-be-seen";

fn declaration(names: &[&str]) -> ExtensionDeclaration {
    let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
    c.secret("api-token", "ACME_NAMED_VALUE", "the API's token");
    c.capability(
        Capability::process("env")
            .argv(&["env"])
            .env(names)
            .why("read the env"),
    );
    c.declaration()
}

fn grant(names: &[&str], roots: &Roots, consent: &dyn Consent) -> Grant {
    Grant::decide(
        &declaration(names),
        &Provenance::third_party(b"x"),
        &EffectsCeiling::default(),
        roots,
        consent,
    )
}

fn session(names: &[&str]) -> PortSession {
    let dir = std::env::temp_dir();
    let roots = Roots::new(&dir, &dir);
    let grant = grant(names, &roots, &MemoryConsent::approving_everything());
    let ports = Ports {
        process: Arc::new(specforge_process::Runner),
        ..testing::ports()
    };
    let secrets = vec!["ACME_NAMED_VALUE".to_string()];
    let call = PortCall::online(&ports, EffectJournal::detached(), roots, grant, secrets);
    PortSession::effectful(EXT, Operation::Collect, Some(call), Limits::CEILING)
}

fn env_with(env: Vec<EnvVar>) -> ProcessRequest {
    ProcessRequest {
        argv: vec!["env".to_string()],
        env,
        ..ProcessRequest::default()
    }
}

#[test]
#[allow(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn a_guests_program_starts_from_nothing_but_what_it_needs_and_declares() {
    // SAFETY: nextest runs each test in its own process, and no thread has started yet.
    unsafe {
        for name in [
            "DATABASE_URL",
            "BASH_ENV",
            "GIT_SSH_COMMAND",
            "CARGO_TARGET_X_RUNNER",
            "ACME_NAMED_VALUE",
            "ACME_DECLARED",
        ] {
            std::env::set_var(name, VALUE);
        }
    }
    let output = session(&["ACME_DECLARED", "ACME_SET", "ACME_NAMED_VALUE"])
        .process_run(env_with(vec![
            EnvVar::set("ACME_SET", "by-the-guest"),
            EnvVar::set("ACME_NAMED_VALUE", "by-the-guest"),
        ]))
        .expect("the capability grants `env`");
    let seen = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.exit_code, Some(0), "{seen}");
    assert!(seen.contains("PATH="), "what a program needs: {seen}");
    assert!(seen.contains("ACME_DECLARED="), "declared: {seen}");
    assert!(seen.contains("ACME_SET=by-the-guest"), "declared: {seen}");
    for name in [
        "DATABASE_URL",
        "BASH_ENV",
        "GIT_SSH_COMMAND",
        "CARGO_TARGET_X_RUNNER",
        "ACME_NAMED_VALUE",
    ] {
        assert!(!seen.contains(name), "{name} reached the program: {seen}");
    }
}

/// A replayed run is the live one: its exit code and both streams.
#[test]
fn a_replay_of_a_run_answers_what_the_live_run_did() {
    const SCRIPT: &str = "echo out; echo err >&2; exit 3";
    let dir = tempfile::tempdir().unwrap();
    let ports = Ports {
        process: Arc::new(specforge_process::Runner),
        ..testing::ports()
    };
    let session = |effects: Effects| {
        let roots = Roots::new(dir.path(), dir.path());
        let mut c = ContributionsBuilder::new(ExtensionMeta::new(EXT, "1.0.0"));
        c.capability(
            Capability::process("sh")
                .argv(&["sh", "-c", SCRIPT])
                .why("run a script"),
        );
        let grant = Grant::decide(
            &c.declaration(),
            &Provenance::third_party(b"x"),
            &EffectsCeiling::default(),
            &roots,
            &MemoryConsent::approving_everything(),
        );
        let call = PortCall::new(effects, &ports, roots, grant, Vec::new());
        PortSession::effectful(EXT, Operation::Collect, Some(call), Limits::CEILING)
    };
    let script = || ProcessRequest {
        argv: ["sh", "-c", SCRIPT].map(String::from).to_vec(),
        ..ProcessRequest::default()
    };
    let journal = || EffectJournal::at(dir.path());

    let live = session(Effects::online(ports.clone(), journal()))
        .process_run(script())
        .unwrap();
    assert_eq!(live.exit_code, Some(3));
    assert_eq!(live.stdout, b"out\n");
    assert_eq!(live.stderr, b"err\n");

    let replayed = session(Effects::replay(journal()))
        .process_run(script())
        .unwrap();
    assert_eq!(replayed, live);
}

#[test]
fn a_guest_setting_a_variable_its_capability_does_not_declare_is_refused() {
    let mut session = session(&["ACME_SET"]);
    for name in ["BASH_ENV", "PATH", "GIT_SSH_COMMAND", "LD_PRELOAD"] {
        let refused = session.process_run(env_with(vec![EnvVar::set(name, "/tmp/evil")]));
        assert!(
            matches!(&refused, Err(HostError::Denied(_))),
            "{name}: {refused:?}"
        );
    }
    let set = session.process_run(env_with(vec![EnvVar::set("ACME_SET", "x")]));
    assert!(set.is_ok(), "{set:?}");
}

#[test]
fn changing_the_declared_names_asks_for_approval_again_and_the_line_shows_them() {
    let dir = std::env::temp_dir();
    let roots = Roots::new(&dir, &dir);
    let consent = MemoryConsent::default();
    let first = grant(&["ACME_A"], &roots, &consent);
    let granted = &first.capabilities()[0];
    assert!(!granted.approved);
    assert!(
        granted.summary().contains("ACME_A"),
        "{}",
        granted.summary()
    );
    consent
        .approve(Approval::Capability(first.key(granted).unwrap()))
        .unwrap();

    let approved = |names: &[&str]| grant(names, &roots, &consent).capabilities()[0].approved;
    assert!(approved(&["ACME_A"]));
    assert!(!approved(&["ACME_A", "ACME_B"]), "a name more asks again");
    assert!(!approved(&[]), "a name less asks again");
}
