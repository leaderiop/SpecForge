use super::*;

fn sizes(entries: &[(&str, u64)]) -> Vec<(String, u64)> {
    entries.iter().map(|&(p, n)| (p.to_string(), n)).collect()
}

#[test]
fn a_blob_over_the_cap_fails_naming_its_path_and_size() {
    let problems = verdict(&sizes(&[
        ("extensions/a/wasm/a.wasm", 10),
        ("extensions/b/wasm/b.wasm", CAP + 1),
    ]));
    assert_eq!(
        problems,
        ["extensions/b/wasm/b.wasm is 2500001 B, over the 2500000 B cap"]
    );
}

#[test]
fn a_blob_at_the_cap_passes() {
    assert!(verdict(&sizes(&[("extensions/a/wasm/a.wasm", CAP)])).is_empty());
}

#[test]
fn measure_reads_each_size_and_leaves_out_a_file_that_is_gone() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.wasm"), [0u8; 3]).unwrap();
    assert_eq!(
        measure(dir.path(), ["a.wasm", "gone.wasm"]),
        sizes(&[("a.wasm", 3)])
    );
}
