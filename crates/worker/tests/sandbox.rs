//! Real-Docker sandbox tests (need a running Docker daemon; they pull `alpine` by digest).

use std::collections::BTreeMap;

use replicata_core::{hash_files, JobSpec};
use replicata_worker::sandbox;

const ALPINE: &str = "sha256:5291449c3df73caf6ed85e649dec1b9e818b39a5d8c871e97afc13e9cd5e8fa8";

fn spec(script: &str, timeout_s: u32) -> JobSpec {
    JobSpec {
        version: 1,
        title: "sandbox test".into(),
        description: String::new(),
        image: "alpine".into(),
        image_digest: ALPINE.into(),
        cmd: vec!["sh".into(), "-c".into(), script.into()],
        env: BTreeMap::from([("GREETING".into(), "hola".into())]),
        inputs: vec![],
        cpus: 1,
        mem_mb: 128,
        timeout_s,
        seed: 7,
    }
}

/// Under the target dir (inside $HOME), because Colima/Docker Desktop only share some host paths.
fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap()
}

async fn run(script: &str, timeout_s: u32) -> (sandbox::RunOutcome, tempfile::TempDir) {
    let dir = tempdir();
    let (inputs, out) = (dir.path().join("inputs"), dir.path().join("out"));
    std::fs::create_dir_all(&inputs).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(inputs.join("in.txt"), b"input").unwrap();
    let outcome = sandbox::run(&sandbox::docker().unwrap(), &spec(script, timeout_s), &inputs, &out).await.unwrap();

    (outcome, dir)
}

#[tokio::test]
async fn writes_results_with_env_seed_and_inputs() {
    let (outcome, dir) = run("echo \"$GREETING $REPLICATA_SEED $(cat /inputs/in.txt)\" > /out/result.txt", 30).await;

    assert_eq!(outcome.exit_code, 0, "{}", outcome.logs);
    let out = dir.path().join("out");
    assert_eq!(std::fs::read_to_string(out.join("result.txt")).unwrap(), "hola 7 input\n");
    let expected = hash_files([("result.txt", &b"hola 7 input\n"[..])]).unwrap();
    assert_eq!(replicata_core::hash_dir(&out).unwrap(), expected);
}

#[tokio::test]
async fn has_no_network() {
    let (outcome, _dir) = run("wget -q -T 3 -O /out/page http://example.com", 30).await;

    assert_ne!(outcome.exit_code, 0);
}

#[tokio::test]
async fn root_filesystem_and_inputs_are_read_only() {
    let (rootfs, _a) = run("echo x > /etc/hacked", 30).await;
    let (inputs, _b) = run("echo x > /inputs/in.txt", 30).await;

    assert_ne!(rootfs.exit_code, 0);
    assert_ne!(inputs.exit_code, 0);
}

#[tokio::test]
async fn timeout_kills_the_container() {
    let started = std::time::Instant::now();
    let (outcome, _dir) = run("sleep 60", 2).await;

    assert!(outcome.timed_out);
    assert!(started.elapsed().as_secs() < 30);
}

#[tokio::test]
async fn non_zero_exit_is_reported() {
    let (outcome, _dir) = run("echo boom >&2; exit 3", 30).await;

    assert_eq!(outcome.exit_code, 3);
    assert!(outcome.logs.contains("boom"));
}

#[tokio::test]
async fn unknown_digest_fails_clearly() {
    let dir = tempdir();
    let mut bad = spec("true", 10);
    bad.image_digest = format!("sha256:{}", "0".repeat(64));

    let err = sandbox::run(&sandbox::docker().unwrap(), &bad, dir.path(), dir.path()).await.unwrap_err();
    assert!(format!("{err:#}").contains("pulling"), "{err:#}");
}
