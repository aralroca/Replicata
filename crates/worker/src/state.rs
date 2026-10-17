//! Crash-safe job state: the salt and progress of every claim live in
//! `<home>/state.json`, so a restarted worker can still reveal (and keep its bond).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Claimed,
    Executed,
    Committed,
    Uploaded,
    Revealed,
    /// The experiment could not be run deterministically (non-zero exit, timeout, bad spec or
    /// inputs). Retrying would fail the same way, so there is nothing to reveal.
    Failed,
    /// Bond recovered with `withdraw_bond` (nobody revealed and the experiment is unresolved).
    Withdrawn,
}

impl Stage {
    /// Still needs work (claimed but not yet revealed).
    pub fn in_progress(self) -> bool {
        self < Stage::Revealed
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub stage: Stage,
    pub salt: Option<String>,
    pub output_hash: Option<String>,
    pub bundle: Option<PathBuf>,
    pub error: Option<String>,
}

impl Job {
    pub fn new(stage: Stage) -> Self {
        Self { stage, salt: None, output_hash: None, bundle: None, error: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct State {
    pub jobs: BTreeMap<String, Job>,
}

impl State {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    /// Atomic write (temp file + rename) so a crash never leaves a torn file. Creates the home
    /// directory if needed: with `--keypair` nothing else has created it yet.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_and_survives_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        let mut state = State::load(&path).unwrap();

        state.jobs.insert(
            "exp".into(),
            Job {
                stage: Stage::Committed,
                salt: Some("aa".into()),
                output_hash: Some("bb".into()),
                bundle: None,
                error: None,
            },
        );
        state.save(&path).unwrap();
        let loaded = State::load(&path).unwrap();
        assert_eq!(loaded.jobs["exp"].stage, Stage::Committed);
        assert_eq!(loaded.jobs["exp"].salt.as_deref(), Some("aa"));
    }

    #[test]
    fn save_creates_a_missing_home() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-created-yet/state.json");
        let mut state = State::default();

        state.jobs.insert("exp".into(), Job::new(Stage::Claimed));
        state.save(&path).unwrap();
        assert_eq!(State::load(&path).unwrap().jobs["exp"].stage, Stage::Claimed);
    }

    #[test]
    fn stages_are_ordered() {
        assert!(Stage::Claimed < Stage::Executed && Stage::Committed < Stage::Revealed);
    }
}
