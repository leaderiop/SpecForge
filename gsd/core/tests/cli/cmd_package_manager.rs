//! `specforge gsd resolve-package-manager <script>` (#5346): the package manager's command, from the files the host
//! reads for it.

use super::commands_support::{gsd, json, refused};
use crate::harness::project;

#[test]
fn resolve_package_manager_reads_the_project_root() {
    let p = project("execute");
    let run = gsd(&p, "resolve-package-manager", &["build"]);
    assert_eq!(run.code, 0, "{}", run.all());
    assert_eq!(
        run.stdout.trim_end(),
        "npm run build",
        "nothing else is printed"
    );
    p.write("yarn.lock", "# yarn\n");
    assert_eq!(
        gsd(&p, "resolve-package-manager", &["test"])
            .stdout
            .trim_end(),
        "yarn test"
    );
    // A lockfile below the root is not consulted, even with no packageManager field to decide: pnpm's would win.
    p.write("sub/pnpm-lock.yaml", "lockfileVersion: 9\n");
    assert_eq!(
        gsd(&p, "resolve-package-manager", &["test"])
            .stdout
            .trim_end(),
        "yarn test"
    );
    // The packageManager field beats the lockfile.
    p.write("package.json", r#"{"packageManager": "bun@1.1.0"}"#);
    assert_eq!(
        gsd(&p, "resolve-package-manager", &["test"])
            .stdout
            .trim_end(),
        "bun run test"
    );
    let answer = json(&gsd(
        &p,
        "resolve-package-manager",
        &["build", "--format", "json"],
    ));
    assert_eq!(answer["command"], "bun run build", "{answer}");
    refused(
        &gsd(&p, "resolve-package-manager", &["a;rm"]),
        "invalid_input",
        &["not a script name"],
    );
}
