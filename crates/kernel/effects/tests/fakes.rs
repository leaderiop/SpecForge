use specforge_effects::http::{Any, SameScheme, get};
use specforge_effects::testing::{
    FakeFs, FakeGit, FakeHttp, FakeProcess, fs_contract, git_contract, http_contract,
    process_contract,
};
use specforge_effects::{
    Http, OutcomeClass, Output, Process, ProcessClass, Redirected, Roots, Shown,
};

#[test]
fn the_fake_process_keeps_the_process_contract() {
    process_contract(&FakeProcess::new());
}

#[test]
fn the_fake_git_keeps_the_git_contract() {
    git_contract(&FakeGit::none(), std::path::Path::new("/nowhere"));
}

#[test]
fn the_fake_process_answers_its_script_and_records_requests() {
    let fake = FakeProcess::new().answering("git", ProcessClass::Exited(0), "abc\r\n");
    let mut request =
        specforge_effects::process::request(["git", "log"], std::time::Duration::from_secs(1));
    let outcome = fake.run(&request, std::path::Path::new("."), Output::Buffer);
    assert!(outcome.success());
    assert_eq!(outcome.text(), "abc\n");
    let mut capture = Vec::new();
    let teed = fake.run(
        &request,
        std::path::Path::new("."),
        Output::Tee {
            capture: &mut capture,
            shown: Shown::Discard,
        },
    );
    assert!(teed.stdout.is_empty());
    assert_eq!(capture, b"abc\r\n");
    request.output_cap = 2;
    let capped = fake.run(&request, std::path::Path::new("."), Output::Buffer);
    assert_eq!(capped.class, ProcessClass::OutputCapped);
    assert_eq!(capped.stdout, b"ab");
    assert_eq!(fake.requests().len(), 3);
}

#[test]
fn the_fake_http_keeps_the_http_contract() {
    http_contract(&FakeHttp::new());
}

#[test]
fn the_fake_http_follows_redirects_as_an_adapter_must() {
    let fake = FakeHttp::new()
        .route("https://a.test/ok", 200, "hello")
        .route("https://a.test/gone", 404, "")
        .redirect("https://a.test/r1", "https://a.test/ok")
        .redirect("https://a.test/down", "http://a.test/ok")
        .redirect("https://a.test/loop", "https://a.test/loop");
    let ok = fake.request(&get("https://a.test/r1"), &Any);
    assert_eq!((ok.class, ok.status), (OutcomeClass::Ok, Some(200)));
    assert_eq!(
        (ok.final_url.as_str(), ok.body.as_slice()),
        ("https://a.test/ok", &b"hello"[..])
    );
    assert_eq!(
        fake.request(&get("https://a.test/gone"), &Any).class,
        OutcomeClass::Status(404)
    );
    assert_eq!(
        fake.request(&get("https://a.test/down"), &Any).class,
        OutcomeClass::Redirect(Redirected::Downgrade)
    );
    assert_eq!(
        fake.request(&get("https://a.test/loop"), &Any).class,
        OutcomeClass::Redirect(Redirected::Limit)
    );
    let refused = FakeHttp::new().redirect("https://a.test/x", "ftp://a.test/y");
    assert_eq!(
        refused.request(&get("https://a.test/x"), &SameScheme).class,
        OutcomeClass::Redirect(Redirected::Refused)
    );
    let mut small = get("https://a.test/ok");
    small.body_cap = 2;
    let capped = fake.request(&small, &Any);
    assert_eq!(
        (capped.class, capped.body.as_slice()),
        (OutcomeClass::Capped, &b"he"[..])
    );
}

#[test]
fn the_fake_fs_keeps_the_fs_contract() {
    let fake = FakeFs::new()
        .with_file("/p/a.txt", "a\n")
        .with_file("/p/.env", "K=v");
    fs_contract(&fake, &Roots::new("/p", "/p/spec"));
}

#[test]
fn the_fake_fs_reads_no_secret_through_a_link() {
    use specforge_effects::{Fs, FsOp, FsRequest};
    let fake = FakeFs::new()
        .with_file("/p/a.txt", "a\n")
        .with_file("/p/.env", "K=v")
        .with_link("/p/notes.txt", ".env")
        .with_link("/p/plain.txt", "a.txt");
    let roots = Roots::new("/p", "/p/spec");
    let read = |path: &str| {
        fake.run(
            &roots,
            &FsRequest {
                root: "project".into(),
                path: path.into(),
                op: FsOp::Read { max_bytes: 1024 },
            },
        )
    };
    let secret = read("notes.txt");
    assert_eq!(secret.class, OutcomeClass::Denied, "{secret:?}");
    assert!(secret.bytes.is_empty());
    let plain = read("plain.txt");
    assert_eq!(plain.class, OutcomeClass::Ok, "{plain:?}");
    assert_eq!(plain.bytes, b"a\n");
}
