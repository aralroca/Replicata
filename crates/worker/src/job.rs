//! One replication, as a resumable state machine:
//! claim → execute (sandbox) → commit → upload bundle → wait for the reveal window → reveal.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use replicata_client::Pubkey;
use replicata_core::{bundle, commitment, from_hex, hash_dir, to_hex, JobSpec};
use reqwest_middleware::ClientWithMiddleware;

use crate::api::{Coordinator, ExperimentView};
use crate::chain::Chain;
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

fn log(id: &str, msg: impl AsRef<str>) {
    println!("[{}] {}", &id[..8.min(id.len())], msg.as_ref());
}

impl Worker {
    fn state_path(&self) -> PathBuf {
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

    /// Drives the job until it is revealed or failed (or until `exit_after`).
    pub async fn process(&self, id: &str) -> anyhow::Result<Stage> {
        let key: Pubkey = id.parse()?;
        let mut job = State::load(&self.state_path())?.jobs.get(id).cloned();

        loop {
            let next = match &job {
                None => self.claim(id, &key).await?,
                Some(j) if j.stage == Stage::Claimed => self.execute(id, j).await,
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
        let view = self.api.experiment(id).await?;
        if view.claim_of(&self.chain.pubkey().to_string()).is_none() {
            self.chain.ensure_token_account().await?;
            let sig = self.chain.claim(key).await.context("claim failed")?;
            log(id, format!("claimed (bond {}) tx {sig}", replicata_core::net::format_usdc(view.bond)));
        }
        Ok(Job { stage: Stage::Claimed, salt: None, output_hash: None, bundle: None, error: None })
    }

    /// Never returns `Err`: a failed run is recorded as `Stage::Failed`.
    async fn execute(&self, id: &str, job: &Job) -> Job {
        match self.try_execute(id).await {
            Ok((hash, bundle)) => Job {
                stage: Stage::Executed,
                salt: Some(hex::encode(rand::random::<[u8; 32]>())),
                output_hash: Some(to_hex(&hash)),
                bundle: Some(bundle),
                error: None,
            },
            Err(err) => {
                log(id, format!("execution failed: {err:#}"));
                Job { stage: Stage::Failed, error: Some(format!("{err:#}")), ..job.clone() }
            }
        }
    }

    async fn try_execute(&self, id: &str) -> anyhow::Result<([u8; 32], PathBuf)> {
        let view = self.api.experiment(id).await?;
        let spec = view.spec.clone().context("the coordinator has no spec for this experiment")?;
        let dir = self.work_dir(id);
        let (inputs_dir, out) = (dir.join("inputs"), dir.join("out"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&inputs_dir)?;
        std::fs::create_dir_all(&out)?;
        verify_spec(&view, &spec)?;
        inputs::fetch_all(&self.http, &spec.inputs, &inputs_dir).await?;
        log(id, format!("running {} …", spec.image_ref()));
        let outcome = sandbox::run(&sandbox::docker()?, &spec, &inputs_dir, &out).await?;
        anyhow::ensure!(!outcome.timed_out, "timed out after {}s", spec.timeout_s);
        anyhow::ensure!(outcome.exit_code == 0, "exit code {}:\n{}", outcome.exit_code, outcome.logs);
        if self.chaos {
            std::fs::write(out.join("chaos.txt"), rand::random::<[u8; 16]>())?;
        }
        package(&out, &dir)
    }

    async fn commit(&self, id: &str, key: &Pubkey, job: &Job) -> anyhow::Result<Job> {
        let (hash, salt) = hash_and_salt(job)?;
        let sig = self.chain.commit(key, commitment(&hash, &salt, &self.chain.pubkey().to_bytes())).await?;
        log(id, format!("committed {} tx {sig}", to_hex(&hash)));

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
        self.wait_for_reveal_window(id).await?;
        let sig = self.chain.reveal(key, hash, salt).await.context("reveal failed")?;
        log(id, format!("revealed tx {sig}"));

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
