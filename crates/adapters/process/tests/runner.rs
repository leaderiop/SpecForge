//! The one process runner (ADR 0335): the Process contract, the deadline and the group kill, the cap, the
//! environment policy, text mode and Windows program resolution.
#![allow(
    clippy::unwrap_used,
    reason = "a test fails on the first unexpected value"
)]

use std::path::Path;
use std::time::Duration;

use specforge_effects::process::request;
use specforge_effects::{Ceiling, Output, Process, ProcessClass, ProcessRequest};
use specforge_process::{
    Located, Passed, Runner, credential, inherited, locate, passed, proxy, secret, without_userinfo,
};

fn run(request: &ProcessRequest) -> specforge_effects::ProcessOutcome {
    Runner.run(request, &std::env::temp_dir(), Output::Buffer)
}

#[cfg(unix)]
fn sh(script: &str, deadline: Duration) -> ProcessRequest {
    request(["sh", "-c", script], deadline)
}

#[test]
fn the_runner_keeps_the_process_contract() {
    specforge_effects::testing::process_contract(&Runner);
}

#[test]
fn a_missing_program_is_not_found() {
    let outcome = run(&request(
        ["specforge-no-such-program"],
        Duration::from_secs(5),
    ));
    assert_eq!(outcome.class, ProcessClass::NotFound, "{outcome:?}");
}

#[test]
fn a_missing_directory_is_not_a_missing_program() {
    let gone = std::env::temp_dir().join("specforge-no-such-directory");
    let outcome = Runner.run(
        &request(["git", "--version"], Duration::from_secs(5)),
        &gone,
        Output::Buffer,
    );
    assert_eq!(outcome.class, ProcessClass::NoDirectory, "{outcome:?}");
}

#[cfg(unix)]
#[test]
fn the_host_build_ceiling_passes_the_request_ceiling() {
    use specforge_effects::Ceiling;
    use specforge_effects::process::{BUILD_CEILING, DEADLINE_CEILING};
    assert!(Ceiling::HostBuild.limit() > Ceiling::Request.limit());
    assert_eq!(Ceiling::Request.limit(), DEADLINE_CEILING);
    assert_eq!(BUILD_CEILING, Duration::from_secs(30 * 60));
    let mut asked = sh("exit 0", Duration::from_secs(5));
    asked.deadline_ms = BUILD_CEILING.as_millis() as u64;
    let outcome = Runner.run_within(&asked, Path::new("/"), Output::Buffer, Ceiling::HostBuild);
    assert_eq!(outcome.class, ProcessClass::Exited(0), "{outcome:?}");
}

/// A capture that cannot be written: a full disk.
#[cfg(unix)]
struct Full;

#[cfg(unix)]
impl std::io::Write for Full {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("no space left"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(unix)]
#[test]
fn a_capture_that_cannot_be_written_stops_the_run_as_uncaptured() {
    let outcome = Runner.run(
        &sh("echo hi", Duration::from_secs(5)),
        &std::env::temp_dir(),
        Output::Tee {
            capture: &mut Full,
            shown: specforge_effects::Shown::Discard,
        },
    );
    assert_eq!(
        outcome.class,
        ProcessClass::Uncaptured("no space left".into()),
        "{outcome:?}"
    );
}

#[cfg(unix)]
#[test]
fn a_grandchild_dies_with_the_group() {
    let outcome = run(&sh("sleep 30 & echo $!; wait", Duration::from_millis(300)));
    assert_eq!(outcome.class, ProcessClass::DeadlineExceeded, "{outcome:?}");
    let pid = outcome.text().trim().to_string();
    assert!(!pid.is_empty(), "{outcome:?}");
    // The killed grandchild is reaped by init: until then `kill -0` still finds it.
    let gone = (0..200).any(|_| {
        let alive = std::process::Command::new("kill")
            .args(["-0", &pid])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success();
        if alive {
            std::thread::sleep(Duration::from_millis(10));
        }
        !alive
    });
    assert!(gone, "the grandchild {pid} outlived the deadline");
}

#[cfg(unix)]
#[test]
fn output_past_the_cap_stops_the_run() {
    let mut capped = sh("yes", Duration::from_secs(20));
    capped.output_cap = 4096;
    let outcome = run(&capped);
    assert_eq!(outcome.class, ProcessClass::OutputCapped, "{outcome:?}");
    assert!(
        outcome.stdout.len() <= 4096 + 64 * 1024,
        "{}",
        outcome.stdout.len()
    );
}

#[cfg(unix)]
#[test]
#[expect(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn a_secret_variable_is_not_passed() {
    // SAFETY: no other thread of this test reads the environment.
    unsafe { std::env::set_var("SPECFORGE_PROBE_TOKEN", "x") };
    let mut listed = request(["env"], Duration::from_secs(10));
    listed.env = vec![specforge_effects::EnvVar::set("SPECFORGE_PROBE_SET", "y")];
    let outcome = run(&listed);
    assert_eq!(outcome.class, ProcessClass::Exited(0), "{outcome:?}");
    let env = outcome.text();
    assert!(!env.contains("SPECFORGE_PROBE_TOKEN"), "{env}");
    assert!(env.contains("SPECFORGE_PROBE_SET=y"), "{env}");
}

#[cfg(unix)]
#[test]
#[allow(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn a_guests_program_inherits_what_it_was_told_and_a_hosts_all_but_a_secret() {
    // SAFETY: no other thread of this test reads the environment.
    unsafe {
        std::env::set_var("SPECFORGE_PROBE_PLAIN", "x");
        std::env::set_var("SPECFORGE_PROBE_DECLARED", "y");
    }
    let host = run(&request(["env"], Duration::from_secs(10))).text();
    assert!(host.contains("SPECFORGE_PROBE_PLAIN=x"), "{host}");

    let mut guests = request(["env"], Duration::from_secs(10));
    guests.inherit = Some(vec!["SPECFORGE_PROBE_DECLARED".to_string()]);
    let guests = run(&guests).text();
    assert!(!guests.contains("SPECFORGE_PROBE_PLAIN"), "{guests}");
    assert!(guests.contains("SPECFORGE_PROBE_DECLARED=y"), "{guests}");
    assert!(guests.contains("PATH="), "{guests}");
}

#[test]
fn what_any_program_inherits_is_a_short_list() {
    for needed in ["PATH", "HOME", "LANG", "LC_ALL", "TMPDIR"] {
        assert!(inherited(needed.as_ref(), &[]), "{needed}");
    }
    for other in ["BASH_ENV", "GIT_SSH_COMMAND", "LD_PRELOAD", "DATABASE_URL"] {
        assert!(!inherited(other.as_ref(), &[]), "{other}");
    }
    assert!(inherited("RUSTFLAGS".as_ref(), &["RUSTFLAGS".to_string()]));
    assert_eq!(inherited("path".as_ref(), &[]), cfg!(windows));
    assert_eq!(inherited("SystemRoot".as_ref(), &[]), cfg!(windows));
}

#[test]
fn a_declared_prefix_passes_every_variable_it_starts() {
    let declared = ["CARGO_*".to_string(), "RUSTUP_*".to_string()];
    for passed in [
        "CARGO_TARGET_DIR",
        "CARGO_PROFILE_DEV_DEBUG",
        "CARGO_HOME",
        "RUSTUP_TOOLCHAIN",
    ] {
        assert!(inherited(passed.as_ref(), &declared), "{passed}");
    }
    for kept in [
        "CARGO",
        "CARGOX_DIR",
        "RUST_LOG",
        "RUSTC_WRAPPER",
        "BASH_ENV",
    ] {
        assert!(!inherited(kept.as_ref(), &declared), "{kept}");
    }
    assert_eq!(inherited("cargo_home".as_ref(), &declared), cfg!(windows));
}

#[cfg(unix)]
#[test]
#[expect(
    unsafe_code,
    reason = "set_var before any thread of this test reads the environment; the test runs in its own process under nextest"
)]
fn the_credential_filter_holds_for_a_guests_program_and_a_hosts_command() {
    // SAFETY: no other thread of this test reads the environment.
    unsafe {
        for (name, value) in CREDENTIALS {
            std::env::set_var(name, value);
        }
        for (name, value) in CLEANED {
            std::env::set_var(name, value);
        }
        for (name, value) in WHOLE {
            std::env::set_var(name, value);
        }
    }
    let env = |inherit: Option<&[&str]>, ceiling: Ceiling| {
        let mut asked = request(["env"], Duration::from_secs(10));
        asked.inherit = inherit.map(|declared| declared.iter().map(ToString::to_string).collect());
        let outcome = Runner.run_within(&asked, &std::env::temp_dir(), Output::Buffer, ceiling);
        assert_eq!(outcome.class, ProcessClass::Exited(0), "{outcome:?}");
        outcome.text()
    };
    let declared: &[&str] = &[
        "CARGO_*",
        "RUSTUP_*",
        "NPM_CONFIG_*",
        "npm_config_*",
        "HTTPS_PROXY",
        "https_proxy",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
        "HTTP_PROXY",
        "NO_PROXY",
        "DATABASE_URL",
        "PWD",
        "OLDPWD",
        // By their exact names, which a person approves: still never passed.
        "CARGO_REGISTRY_TOKEN",
        "CARGO_REGISTRY_CREDENTIAL_PROVIDER",
        "PGPASSWORD",
        "MYSQL_PWD",
        "GH_PAT",
    ];
    // A guest's program (its declared names and prefixes) and a collector's command the host runs (the whole
    // environment): the same filter.
    for (path, seen) in [
        ("guest", env(Some(declared), Ceiling::Request)),
        ("host", env(None, Ceiling::Request)),
    ] {
        for (name, _) in CREDENTIALS {
            assert!(!seen.contains(&format!("{name}=")), "{path} {name}: {seen}");
        }
        for (name, value) in CLEANED {
            let Passed::Without(clean) = passed(name.as_ref(), value.as_ref()) else {
                panic!("{name} is cleaned");
            };
            let clean = clean.to_string_lossy();
            assert!(
                seen.contains(&format!("{name}={clean}\n")),
                "{path} {name}: {seen}"
            );
        }
        for (name, value) in WHOLE {
            assert!(
                seen.contains(&format!("{name}={value}\n")),
                "{path} {name}: {seen}"
            );
        }
    }
    // The host's own build keeps what fetching its crates needs: only a secret is withheld.
    let build = env(None, Ceiling::HostBuild);
    assert!(!build.contains("CARGO_REGISTRY_TOKEN="), "{build}");
    assert!(
        build.contains("CARGO_REGISTRY_CREDENTIAL_PROVIDER=cargo:token"),
        "{build}"
    );
    assert!(
        build.contains("CARGO_HTTP_PROXY=http://user:pass@proxy.test:8080\n"),
        "{build}"
    );
}

/// What the credential filter withholds, by name, on every path.
#[cfg(unix)]
const CREDENTIALS: [(&str, &str); 13] = [
    ("CARGO_REGISTRY_TOKEN", "withheld"),
    ("CARGO_REGISTRIES_X_TOKEN", "withheld"),
    ("CARGO_REGISTRY_CREDENTIAL_PROVIDER", "cargo:token"),
    ("CARGO_REGISTRIES_X_CREDENTIAL_PROVIDER", "cargo:token"),
    ("CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS", "cargo:token"),
    ("CARGO_CREDENTIAL_X", "withheld"),
    ("NPM_CONFIG__AUTH", "withheld"),
    ("NPM_CONFIG__AUTHTOKEN", "withheld"),
    ("npm_config__authToken", "withheld"),
    ("RUSTUP_X_PASSWD", "withheld"),
    ("PGPASSWORD", "withheld"),
    ("MYSQL_PWD", "withheld"),
    ("GH_PAT", "withheld"),
];

/// What passes with its URL's user and password removed.
#[cfg(unix)]
const CLEANED: [(&str, &str); 11] = [
    ("CARGO_HTTP_PROXY", "http://user:pass@proxy.test:8080"),
    ("HTTPS_PROXY", "http://user:pass@proxy.test:8080"),
    ("https_proxy", "http://user:pass@proxy.test:8080"),
    ("ALL_PROXY", "socks5://user:pass@proxy.test:1080"),
    (
        "CARGO_REGISTRIES_X_INDEX",
        "sparse+https://tok3n@registry.test/index/",
    ),
    ("RUSTUP_DIST_SERVER", "https://user:pass@mirror.test"),
    ("RUSTUP_UPDATE_ROOT", "https://user:pass@mirror.test/rustup"),
    ("NPM_CONFIG_REGISTRY", "https://user:pass@npm.test/"),
    // A proxy as curl reads one, with no scheme.
    ("HTTP_PROXY", "user:pass@proxy.test:8080"),
    ("all_proxy", "user:pass@proxy.test:1080"),
    // A password holding an unencoded `#`: the user information runs to the authority's last `@`.
    ("DATABASE_URL", "postgres://user:pa#ss@db.test/x"),
];

/// What passes whole: cargo's own setting, an index and a proxy with no user, a host list that is no proxy, and the
/// working directories, whose names end like a password's.
#[cfg(unix)]
const WHOLE: [(&str, &str); 6] = [
    ("CARGO_SPECFORGE_PROBE", "passed"),
    (
        "CARGO_REGISTRIES_Y_INDEX",
        "sparse+https://registry.test/index/",
    ),
    ("http_proxy", "http://proxy.test:3128"),
    ("NO_PROXY", "a:b@host.test"),
    ("PWD", "/probe/working"),
    ("OLDPWD", "/probe/before"),
];

#[test]
fn a_secret_is_named_like_one() {
    assert!(secret("aws_region".as_ref()) && secret("GH_API_KEY".as_ref()));
    assert!(secret("SIGNING_KEY".as_ref()) && secret("AZURE_CREDENTIALS".as_ref()));
    assert!(secret("GOOGLE_APPLICATION_CREDENTIALS".as_ref()));
    for name in [
        "NPM_CONFIG__AUTH",
        "NPM_CONFIG__AUTHTOKEN",
        "npm_config__authToken",
        "DB_PASSWD",
        "X_SECRET_KEY",
        "X_PRIVATE_KEY",
        "X_API_KEY",
    ] {
        assert!(secret(name.as_ref()), "{name}");
    }
    for name in ["PGPASSWORD", "pgpassword", "MYSQL_PWD", "GH_PAT", "x_pat"] {
        assert!(secret(name.as_ref()), "{name}");
    }
    for name in ["PWD", "OLDPWD", "LD_LIBRARY_PATH", "PATH_INFO"] {
        assert!(!secret(name.as_ref()), "{name}");
    }
    assert!(!secret("TOKENIZER".as_ref()) && !secret("PATH".as_ref()));
    assert!(!secret("AUTHOR".as_ref()) && !secret("OAUTH_SCOPE".as_ref()));
}

#[test]
fn a_cargo_credential_is_named_like_one() {
    for name in [
        "CARGO_REGISTRY_CREDENTIAL_PROVIDER",
        "cargo_registries_my_reg_credential_provider",
        "CARGO_REGISTRY_GLOBAL_CREDENTIAL_PROVIDERS",
        "CARGO_CREDENTIAL_X_TOKEN_FILE",
    ] {
        assert!(credential(name.as_ref()), "{name}");
    }
    for name in [
        "CARGO_REGISTRIES_X_INDEX",
        "CARGO_HTTP_PROXY",
        "CARGO_TARGET_DIR",
    ] {
        assert!(!credential(name.as_ref()), "{name}");
    }
}

#[test]
fn a_url_passes_without_its_user_and_password() {
    let cleaned = |text: &str| without_userinfo(text);
    assert_eq!(
        cleaned("http://user:pass@proxy.test:8080").as_deref(),
        Some("http://proxy.test:8080")
    );
    // A token written as the user, under an HTTP scheme.
    assert_eq!(
        cleaned("sparse+https://tok3n@registry.test/index/").as_deref(),
        Some("sparse+https://registry.test/index/")
    );
    // ssh's account is no secret; its password is.
    assert_eq!(cleaned("ssh://git@github.test/x.git"), None);
    assert_eq!(
        cleaned("ssh://git:pw@github.test/x.git").as_deref(),
        Some("ssh://github.test/x.git")
    );
    // Every URL of a list; an `@` past the host is no user.
    assert_eq!(
        cleaned("https://a:b@one.test/p@q https://two.test/r@s").as_deref(),
        Some("https://one.test/p@q https://two.test/r@s")
    );
    // The user information runs to the authority's last `@`, past an unencoded `#`, `?`, `/` or `@` in a password.
    for (url, clean) in [
        ("postgres://user:pa#ss@db.test/x", "postgres://db.test/x"),
        (
            "postgres://user:pa?ss@db.test/x?y",
            "postgres://db.test/x?y",
        ),
        ("postgres://user:pa/ss@db.test", "postgres://db.test"),
        ("https://user:pa@ss@host.test/a@b", "https://host.test/a@b"),
        ("https://user:pa#s@s@host.test", "https://host.test"),
    ] {
        assert_eq!(cleaned(url).as_deref(), Some(clean), "{url}");
    }
    // A port is no password, a bracketed host no user, and an `@` in the path no user's end.
    for whole in [
        "http://proxy.test:3128",
        "no url here",
        "user@host.test",
        "http://host.test:8080/a@b",
        "http://[::1]:80/a@b",
        "https://registry.npmjs.test/@scope/pkg",
    ] {
        assert_eq!(cleaned(whole), None, "{whole}");
    }
    assert_eq!(
        passed("HTTPS_PROXY".as_ref(), "http://u:p@proxy.test".as_ref()),
        Passed::Without("http://proxy.test".into())
    );
    assert_eq!(
        passed("CARGO_REGISTRY_TOKEN".as_ref(), "x".as_ref()),
        Passed::Withheld
    );
    assert_eq!(
        passed("HTTPS_PROXY".as_ref(), "http://proxy.test".as_ref()),
        Passed::Whole
    );
    // A proxy with no scheme is cleaned as curl reads it; a host list is no proxy.
    assert_eq!(
        passed("https_proxy".as_ref(), "user:pass@proxy.test:8080".as_ref()),
        Passed::Without("proxy.test:8080".into())
    );
    assert_eq!(
        passed("ALL_PROXY".as_ref(), "tok3n@proxy.test".as_ref()),
        Passed::Without("proxy.test".into())
    );
    assert_eq!(
        passed("HTTP_PROXY".as_ref(), "proxy.test:3128".as_ref()),
        Passed::Whole
    );
    assert_eq!(
        passed("NO_PROXY".as_ref(), "a:b@host.test".as_ref()),
        Passed::Whole
    );
    assert!(proxy("CARGO_HTTP_PROXY".as_ref()) && proxy("all_proxy".as_ref()));
    assert!(!proxy("NO_PROXY".as_ref()) && !proxy("no_proxy".as_ref()));
}

#[cfg(unix)]
#[test]
fn a_url_that_is_not_unicode_is_withheld() {
    use std::os::unix::ffi::OsStrExt;
    let value = std::ffi::OsStr::from_bytes(b"http://u:p\xff@proxy.test");
    assert_eq!(passed("HTTPS_PROXY".as_ref(), value), Passed::Withheld);
    let plain = std::ffi::OsStr::from_bytes(b"plain\xff");
    assert_eq!(passed("X".as_ref(), plain), Passed::Whole);
}

#[cfg(unix)]
#[test]
fn a_request_cannot_put_a_secret_back() {
    let mut listed = request(["env"], Duration::from_secs(10));
    listed.env = vec![
        specforge_effects::EnvVar::set("SPECFORGE_PROBE_TOKEN", "x"),
        specforge_effects::EnvVar::set("SPECFORGE_PROBE_CREDENTIALS", "y"),
        specforge_effects::EnvVar::set("SPECFORGE_PROBE_PLAIN", "z"),
    ];
    let env = run(&listed).text();
    assert!(!env.contains("SPECFORGE_PROBE_TOKEN"), "{env}");
    assert!(!env.contains("SPECFORGE_PROBE_CREDENTIALS"), "{env}");
    assert!(env.contains("SPECFORGE_PROBE_PLAIN=z"), "{env}");
}

/// Whether the process `pid` still runs.
#[cfg(unix)]
fn alive(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid])
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[cfg(unix)]
#[test]
fn a_clean_exit_leaves_its_background_child_running() {
    // The child holds stdout open past the program's exit: the run returns after the grace, with what was written.
    let started = std::time::Instant::now();
    let outcome = run(&sh("sleep 30 & echo $!", Duration::from_secs(20)));
    assert_eq!(outcome.class, ProcessClass::Exited(0), "{outcome:?}");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "it waited for the child"
    );
    let pid = outcome.text().trim().to_string();
    assert!(alive(&pid), "the child {pid} was killed after a clean exit");
    std::process::Command::new("kill")
        .arg(&pid)
        .status()
        .unwrap();
}

#[cfg(unix)]
#[test]
fn stop_all_ends_every_running_group() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > {}; wait", pid_file.display());
    let running = std::thread::spawn(move || run(&sh(&script, Duration::from_secs(60))));
    let pid = (0..500)
        .find_map(|_| {
            std::thread::sleep(Duration::from_millis(10));
            std::fs::read_to_string(&pid_file)
                .ok()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
        })
        .expect("the child started");
    assert_eq!(specforge_process::live(), 1);
    specforge_process::stop_all();
    let outcome = running.join().unwrap();
    assert_eq!(outcome.class, ProcessClass::Signalled, "{outcome:?}");
    let gone = (0..200).any(|_| {
        let up = alive(&pid);
        if up {
            std::thread::sleep(Duration::from_millis(10));
        }
        !up
    });
    assert!(gone, "the grandchild {pid} outlived stop_all");
    assert_eq!(specforge_process::live(), 0);
}

#[cfg(unix)]
#[test]
fn text_mode_reads_crlf_as_lf() {
    let mut printed = sh("printf 'a\\r\\nb\\r\\n'", Duration::from_secs(10));
    printed.text = true;
    assert_eq!(run(&printed).stdout, b"a\nb\n");
    printed.text = false;
    assert_eq!(run(&printed).stdout, b"a\r\nb\r\n");
}

#[test]
fn a_cmd_shim_resolves_through_pathext() {
    let dir = tempfile::tempdir().unwrap();
    // Spelled as `PATHEXT` spells it, so a disk whose names keep their case (Linux) finds it too.
    std::fs::write(dir.path().join("shim.CMD"), "@exit /b 3\r\n").unwrap();
    let path = std::env::join_paths([Path::new("/nowhere"), dir.path()]).unwrap();
    let base = std::env::temp_dir();
    // PATHEXT spells the extension (`.CMD`): Windows file names ignore case, so compare as it does.
    let found = |pathext: &str| match locate("shim", Some(&path), Some(pathext.as_ref()), &base) {
        Located::At(p) => Some(p.to_string_lossy().to_lowercase()),
        _ => None,
    };
    let shim = Some(dir.path().join("shim.CMD").to_string_lossy().to_lowercase());
    assert_eq!(found(".COM;.EXE;.BAT;.CMD"), shim);
    assert_eq!(
        locate("other", Some(&path), Some(".CMD".as_ref()), &base),
        Located::Missing
    );
    // The request's PATHEXT decides: one that names no `.cmd` finds no shim.
    assert_eq!(found(".EXE"), None);
    assert_eq!(found(".BAT;.CMD"), shim);
}

/// A bare name resolves only through absolute `PATH` entries: one planted where a relative or an empty entry points
/// (from the directory the program runs in) is never chosen, and is named so the refusal can say which entry.
#[test]
fn a_bare_name_skips_relative_and_empty_path_entries() {
    let base = tempfile::tempdir().unwrap();
    let planted = base.path().join("bin");
    std::fs::create_dir_all(&planted).unwrap();
    for name in ["tool", "tool.cmd"] {
        let file = planted.join(name);
        std::fs::write(&file, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    let empty = tempfile::tempdir().unwrap();
    let pathext = specforge_process::pathext(None);
    let located =
        |path: &std::ffi::OsStr, base: &Path| locate("tool", Some(path), pathext.as_deref(), base);
    let relative = std::env::join_paths([Path::new("bin"), empty.path()]).unwrap();
    assert_eq!(
        located(&relative, base.path()),
        Located::Relative("bin".into())
    );
    // An empty entry is the directory the program runs in.
    let blank = std::env::join_paths([Path::new(""), empty.path()]).unwrap();
    assert_eq!(located(&blank, &planted), Located::Relative("".into()));
    // An absolute entry after a relative one answers the name.
    let installed = std::env::join_paths([Path::new("bin"), &planted]).unwrap();
    assert!(
        matches!(located(&installed, empty.path()), Located::At(at) if at.starts_with(&planted))
    );
    // A name with a directory is the caller's own choice, taken as written.
    let named = planted.join("tool");
    assert!(matches!(
        locate(
            &named.to_string_lossy(),
            None,
            pathext.as_deref(),
            empty.path()
        ),
        Located::At(_)
    ));
    let refusal = Located::refusal("tool", Path::new("bin"));
    assert!(
        refusal.contains("`bin`") && refusal.contains("absolute directory on PATH"),
        "{refusal}"
    );
    // The runner refuses it rather than run it.
    let outcome = Runner.run(
        &ProcessRequest {
            env: vec![specforge_effects::EnvVar {
                name: "PATH".into(),
                value: Some(relative.to_string_lossy().into_owned()),
            }],
            ..request(["tool"], Duration::from_secs(5))
        },
        base.path(),
        Output::Buffer,
    );
    assert!(
        matches!(&outcome.class, ProcessClass::CannotStart(why) if why.contains("relative PATH entry")),
        "{outcome:?}"
    );
}

#[cfg(windows)]
#[test]
fn a_cmd_shim_on_path_runs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("shim.cmd"), "@exit /b 3\r\n").unwrap();
    let outcome = Runner.run(
        &request(
            [dir.path().join("shim").to_string_lossy()],
            Duration::from_secs(10),
        ),
        dir.path(),
        Output::Buffer,
    );
    assert_eq!(outcome.class, ProcessClass::Exited(3), "{outcome:?}");
}
