//! Review finding 7.2: the tests never read the machine's global or system git configuration.
//! `rust/.cargo/config.toml` exports the isolation to every test process; this fails when that
//! setting is dropped, and shows a fixture commit is made under it.

mod common;

#[test]
fn the_test_environment_isolates_git_from_the_machines_configuration() {
    assert!(
        common::git_environment_is_isolated(),
        "GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM=1 must reach every test \
         (rust/.cargo/config.toml [env]); run the tests through cargo"
    );
}

#[test]
fn a_fixture_commit_ignores_a_signing_configuration_the_machine_sets() {
    let dir = common::TempDir::new("urzua-git-isolation");
    // A signing helper that always fails. If it were read, the commit would fail.
    let mut cmd = common::isolated_git();
    cmd.args(["init", "-q"]).current_dir(&dir);
    assert!(cmd.status().unwrap().success());
    std::fs::write(dir.join("f"), "x\n").unwrap();
    common::git(&dir, &["add", "-A"]);
    let home = common::TempDir::new("urzua-git-isolation-home");
    std::fs::write(
        home.join(".gitconfig"),
        "[commit]\n\tgpgsign = true\n[gpg]\n\tprogram = /nonexistent/signer\n",
    )
    .unwrap();
    let status = common::isolated_git()
        .args(["commit", "-q", "-m", "fixture"])
        .env("HOME", &*home)
        .env("XDG_CONFIG_HOME", &*home)
        .current_dir(&dir)
        .status()
        .unwrap();
    assert!(
        status.success(),
        "the machine's signing configuration leaked into a fixture commit"
    );
}
