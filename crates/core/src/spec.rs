//! `JobSpec`: everything a worker needs to replicate an experiment.
//!
//! The on-chain `Experiment.spec_hash` is `sha256(canonical_json(spec))`, where the
//! canonical JSON has object keys sorted bytewise and no insignificant whitespace.
//! `serde_json` (without `preserve_order`) serialises `Value` maps sorted, and the web app
//! reproduces the same bytes (checked by `vectors/spec.json`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SPEC_VERSION: u32 = 1;
pub const MAX_TIMEOUT_S: u32 = 3_600;
pub const MAX_MEM_MB: u32 = 16_384;
pub const MAX_CPUS: u32 = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Input {
    /// Where to download the input from (plain HTTP(S) or an x402-paid URL).
    pub url: String,
    /// Lowercase hex sha256 of the file contents.
    pub sha256: String,
    /// Relative path inside the container's `/inputs` mount.
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobSpec {
    pub version: u32,
    pub title: String,
    pub description: String,
    /// Image repository, e.g. `ghcr.io/aralroca/replicata-pi`.
    pub image: String,
    /// `sha256:<64 hex>`: the image is always pulled and run by digest.
    pub image_digest: String,
    pub cmd: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub inputs: Vec<Input>,
    pub cpus: u32,
    pub mem_mb: u32,
    pub timeout_s: u32,
    pub seed: u64,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SpecError {
    #[error("unsupported spec version {0}")]
    Version(u32),
    #[error("title must be 1..=120 characters")]
    Title,
    #[error("image_digest must look like sha256:<64 hex chars>")]
    Digest,
    #[error("image must be a non-empty repository name without tag or digest")]
    Image,
    #[error("cmd must not be empty")]
    Cmd,
    #[error("input {0}: sha256 must be 64 hex chars")]
    InputHash(usize),
    #[error("input {0}: path must be relative, non-empty and without `..`")]
    InputPath(usize),
    #[error("input {0}: url must be http(s)")]
    InputUrl(usize),
    #[error("resources out of range (cpus 1..={MAX_CPUS}, mem_mb 64..={MAX_MEM_MB}, timeout_s 1..={MAX_TIMEOUT_S})")]
    Resources,
    #[error("invalid JSON: {0}")]
    Json(String),
}

impl JobSpec {
    /// Canonical JSON bytes (sorted keys, compact).
    pub fn canonical_json(&self) -> Vec<u8> {
        let value = serde_json::to_value(self).expect("JobSpec always serialises");

        serde_json::to_vec(&value).expect("Value always serialises")
    }

    /// `sha256(canonical_json)`, stored on-chain as `Experiment.spec_hash`.
    pub fn spec_hash(&self) -> [u8; 32] {
        Sha256::digest(self.canonical_json()).into()
    }

    /// The 32 raw bytes of `image_digest`, stored on-chain as `Experiment.image_digest`.
    pub fn image_digest_bytes(&self) -> Option<[u8; 32]> {
        self.image_digest.strip_prefix("sha256:").and_then(crate::from_hex)
    }

    /// `image@sha256:...`, the only reference the worker ever pulls.
    pub fn image_ref(&self) -> String {
        format!("{}@{}", self.image, self.image_digest)
    }

    pub fn parse(json: &[u8]) -> Result<Self, SpecError> {
        let spec: Self = serde_json::from_slice(json).map_err(|e| SpecError::Json(e.to_string()))?;

        spec.validate()?;
        Ok(spec)
    }

    pub fn validate(&self) -> Result<(), SpecError> {
        if self.version != SPEC_VERSION {
            return Err(SpecError::Version(self.version));
        }
        if self.title.trim().is_empty() || self.title.chars().count() > 120 {
            return Err(SpecError::Title);
        }
        if self.image.is_empty() || self.image.contains('@') || self.image.contains(char::is_whitespace) {
            return Err(SpecError::Image);
        }
        if self.image_digest_bytes().is_none() {
            return Err(SpecError::Digest);
        }
        if self.cmd.is_empty() {
            return Err(SpecError::Cmd);
        }
        self.inputs.iter().enumerate().try_for_each(|(i, input)| input.validate(i))?;
        let resources_ok = (1..=MAX_CPUS).contains(&self.cpus)
            && (64..=MAX_MEM_MB).contains(&self.mem_mb)
            && (1..=MAX_TIMEOUT_S).contains(&self.timeout_s);
        if !resources_ok {
            return Err(SpecError::Resources);
        }
        Ok(())
    }
}

impl Input {
    fn validate(&self, index: usize) -> Result<(), SpecError> {
        if crate::from_hex(&self.sha256).is_none() {
            return Err(SpecError::InputHash(index));
        }
        if !is_safe_relative_path(&self.path) {
            return Err(SpecError::InputPath(index));
        }
        if !(self.url.starts_with("https://") || self.url.starts_with("http://")) {
            return Err(SpecError::InputUrl(index));
        }
        Ok(())
    }
}

/// Relative, non-empty, `/`-separated, no `.`/`..`/empty segments.
pub fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn sample() -> JobSpec {
        JobSpec {
            version: 1,
            title: "Monte-Carlo π".into(),
            description: "Estimate π with 10M samples".into(),
            image: "ghcr.io/aralroca/replicata-pi".into(),
            image_digest: format!("sha256:{}", "ab".repeat(32)),
            cmd: vec!["python".into(), "/app/main.py".into()],
            env: BTreeMap::from([("OMP_NUM_THREADS".into(), "1".into())]),
            inputs: vec![],
            cpus: 1,
            mem_mb: 512,
            timeout_s: 120,
            seed: 42,
        }
    }

    #[test]
    fn canonical_json_sorts_keys_and_is_compact() {
        let json = String::from_utf8(sample().canonical_json()).unwrap();

        assert!(json.starts_with("{\"cmd\":[\"python\""));
        assert!(!json.contains(": "));
        assert!(json.find("\"cpus\"").unwrap() < json.find("\"description\"").unwrap());
    }

    #[test]
    fn spec_hash_ignores_input_key_order() {
        let reordered = r#"{"version":1,"seed":42,"timeout_s":120,"mem_mb":512,"cpus":1,"inputs":[],
            "env":{"OMP_NUM_THREADS":"1"},"cmd":["python","/app/main.py"],
            "image_digest":"sha256:abababababababababababababababababababababababababababababababab",
            "image":"ghcr.io/aralroca/replicata-pi","description":"Estimate π with 10M samples","title":"Monte-Carlo π"}"#;

        assert_eq!(JobSpec::parse(reordered.as_bytes()).unwrap().spec_hash(), sample().spec_hash());
    }

    #[test]
    fn spec_hash_changes_with_any_field() {
        let mut other = sample();

        other.seed = 43;
        assert_ne!(other.spec_hash(), sample().spec_hash());
    }

    #[test]
    fn validation_errors() {
        type Mutation = fn(&mut JobSpec);
        let cases: Vec<(Mutation, SpecError)> = vec![
            (|s| s.version = 2, SpecError::Version(2)),
            (|s| s.title = " ".into(), SpecError::Title),
            (|s| s.image_digest = "sha256:xyz".into(), SpecError::Digest),
            (|s| s.image = "alpine@sha256".into(), SpecError::Image),
            (|s| s.cmd.clear(), SpecError::Cmd),
            (|s| s.timeout_s = 0, SpecError::Resources),
            (|s| s.mem_mb = 1, SpecError::Resources),
            (
                |s| s.inputs.push(Input { url: "https://x".into(), sha256: "00".repeat(32), path: "../etc".into() }),
                SpecError::InputPath(0),
            ),
            (
                |s| s.inputs.push(Input { url: "ftp://x".into(), sha256: "00".repeat(32), path: "a".into() }),
                SpecError::InputUrl(0),
            ),
            (
                |s| s.inputs.push(Input { url: "https://x".into(), sha256: "zz".into(), path: "a".into() }),
                SpecError::InputHash(0),
            ),
        ];

        for (mutate, expected) in cases {
            let mut spec = sample();

            mutate(&mut spec);
            assert_eq!(spec.validate(), Err(expected));
        }
        assert_eq!(sample().validate(), Ok(()));
    }

    #[test]
    fn image_ref_uses_digest() {
        assert_eq!(sample().image_ref(), format!("ghcr.io/aralroca/replicata-pi@sha256:{}", "ab".repeat(32)));
    }
}
