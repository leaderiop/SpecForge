use std::fs::{self, File};
use std::time::{Duration, SystemTime};

use super::prune;

const HOUR: Duration = Duration::from_secs(3600);

fn written(dir: &std::path::Path, name: &str, age: Duration) {
    let file = File::create(dir.join(name)).unwrap();
    file.set_len(10).unwrap();
    file.set_modified(SystemTime::now() - age).unwrap();
}

#[test]
fn only_an_old_debug_object_goes() {
    let dir = tempfile::tempdir().unwrap();
    written(dir.path(), "a-0123456789abcdef.a.1x-cgu.0.rcgu.o", 2 * HOUR);
    written(
        dir.path(),
        "b-0123456789abcdef.b.2y-cgu.0.rcgu.o",
        Duration::ZERO,
    );
    written(dir.path(), "liba-0123456789abcdef.rlib", 2 * HOUR);

    assert_eq!(prune(dir.path(), HOUR, true), 10, "a dry run counts it");
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        3,
        "a dry run keeps it"
    );

    assert_eq!(prune(dir.path(), HOUR, false), 10);
    let mut left: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(
        left,
        [
            "b-0123456789abcdef.b.2y-cgu.0.rcgu.o",
            "liba-0123456789abcdef.rlib"
        ]
    );
}

#[test]
fn a_missing_directory_frees_nothing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(prune(&dir.path().join("deps"), HOUR, false), 0);
}
