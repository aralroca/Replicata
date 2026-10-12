//! One replication, as a resumable state machine:
//! claim → execute (sandbox) → commit → upload bundle → wait for the reveal window → reveal.
//!
//! Every on-chain step first checks the claim account, so resuming after a crash (or after a
//! transaction that landed but whose confirmation was lost) never repeats a step. Only
//! deterministic failures are terminal; anything transient returns `Err` and is retried.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use replicata_client::{ClaimState, Pubkey};
use replicata_core::{bundle, commitment, from_hex, hash_dir, to_hex, JobSpec};
use reqwest_middleware::ClientWithMiddleware;

use crate::api::{Coordinator, ExperimentView};
use crate::chain::Chain;
use crate::inputs::InputMismatch;
use crate::state::{Job, Stage, State};
use crate::{inputs, sandbox};

pub struct Worker {
    pub chain: Chain,
    pub api: Coordinator,
    pub http: ClientWithMiddleware,
    pub home: PathBuf,
    /// Test hook: tamper with the output so this worker disagrees with the others.
    pub chaos: bool,
    /// Test hook: stop right after reaching this stage (simulates a crash).
    pub exit_after: Option<Stage>,
}

/// A failure that would happen again on retry (bad spec, non-zero exit, timeout...).
#[derive(Debug)]
pub struct Deterministic(pub String);

impl std::fmt::Display for Deterministic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Deterministic {}

fn deterministic(msg: impl Into<String>) -> anyhow::Error {
    Deterministic(msg.into()).into()
}

fn is_deterministic(err: &anyhow::Error) -> bool {
    err.is::<Deterministic>() || err.is::<InputMismatch>()
}

pub fn log(id: &str, msg: impl AsRef<str>) {
    println!("[{}] {}", &id[..8.min(id.len())], msg.as_ref());
}

impl Worker {
    pub fn state_path(&self) -> PathBuf {
        self.home.join("state.json")
    }

    fn work_dir(&self, id: &str) -> PathBuf {
        self.home.join("work").join(id)
    }

    fn save(&self, id: &str, job: &Job) -> anyhow::Result<()> {
        let mut state = State::load(&self.state_path())?;
        state.jobs.insert(id.to_string(), job.clone());
        state.save(&self.state_path())
    }

    /// Drives the job until it is revealed or failed (or until `exit_after`). Returns early,
    /// without doing anything, once the experiment is no longer open.
    pub async fn process(&self, id: &str) -> anyhow::Result<Stage> {
        let key: Pubkey = id.parse()?;
        let mut job = State::load(&self.state_path())?.jobs.get(id).cloned();
        let open = self.api.experiment(id).await?.status == "open";
        if !open {
            return job.map(|j| j.stage).context("the experiment is no longer open");
        }

        loop {
            let next = match &job {
                None => self.claim(id, &key).await?,
                Some(j) if j.stage == Stage::Claimed => self.execute(id, j).await?,
                Some(j) if j.stage == Stage::Executed => self.commit(id, &key, j).await?,
                Some(j) if j.stage == Stage::Committed => self.upload(id, j).await?,
                Some(j) if j.stage == Stage::Uploaded => self.reveal(id, &key, j).await?,
                Some(j) => return Ok(j.stage),
            };
            self.save(id, &next)?;
            log(id, format!("stage: {:?}", next.stage));
            if self.exit_after == Some(next.stage) {
                return Ok(next.stage);
            }
            job = Some(next);
        }
    }

    async fn claim(&self, id: &str, key: &Pubkey) -> anyhow::Result<Job> {
        if self.chain.my_claim(key).await?.is_none() {
            let view = self.api.experiment(id).await?;
            self.chain.ensure_token_account().await?;
            let sig = self.chain.claim(key).await.context("claim failed")?;
            log(id, format!("claimed (bond {}) tx {sig}", replicata_core::net::format_usdc(view.bond)));
        }
        Ok(Job { stage: Stage::Claimed, salt: None, output_hash: None, bundle: None, error: None })
    }

    /// `Failed` for deterministic failures; transient ones propagate (the job stays `Claimed`).
    async fn execute(&self, id: &str, job: &Job) -> anyhow::Result<Job> {
        match self.try_execute(id).await {
            Ok((hash, bundle)) => Ok(Job {
                stage: Stage::Executed,
                salt: Some(hex::encode(rand::random::<[u8; 32]>())),
                output_hash: Some(to_hex(&hash)),
                bundle: Some(bundle),
                error: None,
            }),
            Err(err) if is_deterministic(&err) => {
                log(id, format!("execution failed: {err:#}"));
                Ok(Job { stage: Stage::Failed, error: Some(format!("{err:#}")), ..job.clone() })
            }
            Err(err) => Err(err.context("execution interrupted (will retry)")),
        }
    }

    async fn try_execute(&self, id: &str) -> anyhow::Result<([u8; 32], PathBuf)> {
        let view = self.api.experiment(id).await?;
        let spec = view.spec.clone().context("the coordinator has no spec for this experiment yet")?;
        verify_spec(&view, &spec).map_err(|e| deterministic(format!("{e:#}")))?;
        let dir = self.work_dir(id);
        let (inputs_dir, out) = (dir.join("inputs"), dir.join("out"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&inputs_dir)?;
        std::fs::create_dir_all(&out)?;
        inputs::fetch_all(&self.http, &spec.inputs, &inputs_dir).await?;
        log(id, format!("running {} …", spec.image_ref()));
        let outcome = sandbox::run(&sandbox::docker()?, &spec, &inputs_dir, &out).await?;
        if outcome.timed_out {
            return Err(deterministic(format!("timed out after {}s", spec.timeout_s)));
        }
        if outcome.exit_code != 0 {
            return Err(deterministic(format!("exit code {}:\n{}", outcome.exit_code, outcome.logs)));
        }
        if self.chaos {
            std::fs::write(out.join("chaos.txt"), rand::random::<[u8; 16]>())?;
        }
        package(&out, &dir).map_err(|e| deterministic(format!("invalid output: {e:#}")))
    }

    async fn commit(&self, id: &str, key: &Pubkey, job: &Job) -> anyhow::Result<Job> {
        let (hash, salt) = hash_and_salt(job)?;
        let ours = commitment(&hash, &salt, &self.chain.pubkey().to_bytes());
        let claim = self.chain.my_claim(key).await?.context("claim account not found")?;
        if claim.state == ClaimState::Claimed {
            let sig = self.chain.commit(key, ours).await?;
            log(id, format!("committed {} tx {sig}", to_hex(&hash)));
        } else {
            anyhow::ensure!(claim.commitment == ours, "on-chain commitment differs from the saved one");
        }
        Ok(Job { stage: Stage::Committed, ..job.clone() })
    }

    async fn upload(&self, id: &str, job: &Job) -> anyhow::Result<Job> {
        let (hash, _) = hash_and_salt(job)?;
        let bundle = std::fs::read(job.bundle.as_ref().context("missing bundle path")?)?;
        self.api.upload_bundle(id, &self.chain.keypair, &hash, bundle).await?;

        Ok(Job { stage: Stage::Uploaded, ..job.clone() })
    }

    async fn reveal(&self, id: &str, key: &Pubkey, job: &Job) -> anyhow::Result<Job> {
        let (hash, salt) = hash_and_salt(job)?;
        let claim = self.chain.my_claim(key).await?.context("claim account not found")?;
        if claim.state == ClaimState::Committed {
            self.wait_for_reveal_window(id).await?;
            let sig = self.chain.reveal(key, hash, salt).await.context("reveal failed")?;
            log(id, format!("revealed tx {sig}"));
        }
        Ok(Job { stage: Stage::Revealed, ..job.clone() })
    }

    async fn wait_for_reveal_window(&self, id: &str) -> anyhow::Result<()> {
        loop {
            let view = self.api.experiment(id).await?;
            match view.phase.as_str() {
                "revealing" => return Ok(()),
                "claiming" => tokio::time::sleep(Duration::from_secs(1)).await,
                other => anyhow::bail!("reveal window is over (phase {other})"),
            }
        }
    }

    /// Recovers the bond of an unresolved experiment nobody revealed.
    pub async fn withdraw(&self, id: &str) -> anyhow::Result<Stage> {
        let sig = self.chain.withdraw_bond(&id.parse()?).await?;
        let job = State::load(&self.state_path())?.jobs.get(id).cloned();
        let withdrawn = Job {
            stage: Stage::Withdrawn,
            ..job.unwrap_or(Job { stage: Stage::Withdrawn, salt: None, output_hash: None, bundle: None, error: None })
        };
        self.save(id, &withdrawn)?;
        log(id, format!("bond withdrawn tx {sig}"));

        Ok(Stage::Withdrawn)
    }
}

/// The spec served by the coordinator must hash to what is on-chain.
fn verify_spec(view: &ExperimentView, spec: &JobSpec) -> anyhow::Result<()> {
    spec.validate()?;
    let expected = view.spec_hash.as_str();
    anyhow::ensure!(to_hex(&spec.spec_hash()) == expected, "spec does not match the on-chain spec_hash {expected}");
    Ok(())
}

fn package(out: &Path, dir: &Path) -> anyhow::Result<([u8; 32], PathBuf)> {
    let hash = hash_dir(out)?;
    let path = dir.join("bundle.tar.gz");
    std::fs::write(&path, bundle::pack_dir(out)?)?;

    Ok((hash, path))
}

fn hash_and_salt(job: &Job) -> anyhow::Result<([u8; 32], [u8; 32])> {
    let hash = job.output_hash.as_deref().and_then(from_hex).context("missing output hash")?;
    let salt = job.salt.as_deref().and_then(from_hex).context("missing salt")?;

    Ok((hash, salt))
}
