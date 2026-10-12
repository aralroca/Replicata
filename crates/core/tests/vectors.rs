//! Cross-language test vectors (`vectors/core.json`), consumed by the program tests and
//! the web app. Run with `BLESS=1` to regenerate after an intentional format change.

use std::path::PathBuf;

use replicata_core::{commitment, hash_files, to_hex, JobSpec};
use serde_json::{json, Value};

fn vectors_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vectors/core.json")
}

fn spec_cases() -> Vec<Value> {
    vec![
        json!({
            "version": 1, "title": "Monte-Carlo π", "description": "Estimate π\nwith \"quotes\" and ünïcode",
            "image": "ghcr.io/aralroca/replicata-pi",
            "image_digest": format!("sha256:{}", "0f".repeat(32)),
            "cmd": ["python", "/app/main.py", "--samples", "1000000"],
            "env": {"PYTHONHASHSEED": "0", "OMP_NUM_THREADS": "1"},
            "inputs": [{"url": "https://example.org/data.csv", "sha256": "aa".repeat(32), "path": "data/data.csv"}],
            "cpus": 1, "mem_mb": 512, "timeout_s": 300, "seed": 42
        }),
        json!({
            "version": 1, "title": "t", "description": "", "image": "alpine",
            "image_digest": format!("sha256:{}", "12".repeat(32)), "cmd": ["sh", "-c", "echo 1 > /out/x"],
            "env": {}, "inputs": [], "cpus": 16, "mem_mb": 64, "timeout_s": 1, "seed": 18446744073709551615u64
        }),
    ]
}

fn output_cases() -> Vec<Vec<(&'static str, &'static str)>> {
    vec![vec![], vec![("result.json", "{\"pi\":3.14159}")], vec![("b.txt", "2"), ("a/z.txt", "1"), ("a.txt", "0")]]
}

fn build() -> Value {
    let specs: Vec<Value> = spec_cases()
        .into_iter()
        .map(|spec| {
            let parsed: JobSpec = serde_json::from_value(spec.clone()).unwrap();
            json!({
                "spec": spec,
                "canonical": String::from_utf8(parsed.canonical_json()).unwrap(),
                "spec_hash": to_hex(&parsed.spec_hash()),
            })
        })
        .collect();
    let outputs: Vec<Value> = output_cases()
        .into_iter()
        .map(|files| {
            let hash = hash_files(files.iter().map(|(n, c)| (*n, c.as_bytes()))).unwrap();
            json!({ "files": files, "hash": to_hex(&hash) })
        })
        .collect();
    let commit = commitment(&[0x11; 32], &[0x22; 32], &[0x33; 32]);

    json!({
        "specs": specs,
        "outputs": outputs,
        "commitments": [{
            "output_hash": "11".repeat(32), "salt": "22".repeat(32), "worker": "33".repeat(32),
            "commitment": to_hex(&commit)
        }]
    })
}

#[test]
fn vectors_are_up_to_date() {
    let expected = build();
    let path = vectors_path();

    if std::env::var("BLESS").is_ok() {
        std::fs::write(&path, serde_json::to_string_pretty(&expected).unwrap() + "\n").unwrap();
    }
    let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(on_disk, expected, "vectors/core.json is stale: run `BLESS=1 cargo test -p replicata-core`");
}
