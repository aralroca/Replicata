//! Determinism of the demo experiments (real Docker).
//!
//! Reproducible examples must hash identically run after run **and** across CPU
//! architectures: `examples/experiments/<name>/expected_hash` was recorded on arm64 and CI
//! checks it on x86_64. `BLESS=1` rewrites the expected hashes.

use std::path::PathBuf;

use replicata_core::{hash_dir, to_hex, JobSpec};
use replicata_worker::sandbox;

fn example(name: &str) -> (JobSpec, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/experiments").join(name);
    let spec = JobSpec::parse(&std::fs::read(dir.join("spec.json")).unwrap()).unwrap();

    (spec, dir)
}

async fn run_once(spec: &JobSpec) -> String {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (inputs, out) = (dir.path().join("inputs"), dir.path().join("out"));
    std::fs::create_dir_all(&inputs).unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let outcome = sandbox::run(&sandbox::docker().unwrap(), spec, &inputs, &out).await.unwrap();
    assert_eq!(outcome.exit_code, 0, "{}", outcome.logs);

    to_hex(&hash_dir(&out).unwrap())
}

async fn assert_reproducible(name: &str) {
    let (spec, dir) = example(name);
    let first = run_once(&spec).await;
    let second = run_once(&spec).await;
    assert_eq!(first, second, "{name} is not reproducible on this machine");

    let expected_path = dir.join("expected_hash");
    if std::env::var("BLESS").is_ok() {
        std::fs::write(&expected_path, format!("{first}\n")).unwrap();
    }
    let expected = std::fs::read_to_string(&expected_path).unwrap();
    assert_eq!(first, expected.trim(), "{name}: hash differs from the one recorded on another architecture");
}

#[tokio::test]
async fn pi_montecarlo_is_reproducible() {
    assert_reproducible("pi-montecarlo").await;
}

#[tokio::test]
async fn lorenz_int_is_reproducible() {
    assert_reproducible("lorenz-int").await;
}

#[tokio::test]
async fn non_reproducible_really_is_not() {
    let (spec, _) = example("non-reproducible");

    assert_ne!(run_once(&spec).await, run_once(&spec).await);
}
