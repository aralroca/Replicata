//! Deterministic Docker sandbox.
//!
//! The image is pulled **by digest**; the container has no network, a read-only root
//! filesystem, no capabilities, `no-new-privileges`, CPU/memory/pids limits and a hard
//! timeout. Inputs are mounted read-only at `/inputs`, results are written to `/out`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use bollard::models::{ContainerCreateBody, HostConfig};
use bollard::query_parameters::{
    CreateContainerOptionsBuilder, CreateImageOptionsBuilder, KillContainerOptionsBuilder, LogsOptionsBuilder,
    RemoveContainerOptionsBuilder,
};
use bollard::Docker;
use futures_util::StreamExt;
use replicata_core::JobSpec;

#[derive(Debug)]
pub struct RunOutcome {
    pub exit_code: i64,
    pub timed_out: bool,
    pub logs: String,
}

/// Well-known Docker sockets (Docker Desktop, Colima, OrbStack, Rancher Desktop).
fn socket_candidates() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());

    [".docker/run/docker.sock", ".colima/default/docker.sock", ".orbstack/run/docker.sock", ".rd/docker.sock"]
        .iter()
        .map(|p| home.join(p))
        .chain([PathBuf::from("/var/run/docker.sock")])
        .collect()
}

/// Connects to `DOCKER_HOST` if set (unix:// or tcp://, e.g. a CI Docker-in-Docker service),
/// otherwise to the first existing well-known socket.
pub fn docker() -> anyhow::Result<Docker> {
    if std::env::var_os("DOCKER_HOST").is_some() {
        return Docker::connect_with_defaults().context("cannot reach Docker at DOCKER_HOST");
    }
    let socket = socket_candidates()
        .into_iter()
        .find(|p| p.exists())
        .context("cannot find a Docker socket (is Docker running?)")?;

    Docker::connect_with_unix(&socket.to_string_lossy(), 120, bollard::API_DEFAULT_VERSION)
        .context("cannot reach Docker")
}

/// Pulls `image@sha256:...` unless it is already present locally.
pub async fn ensure_image(docker: &Docker, image_ref: &str) -> anyhow::Result<()> {
    if docker.inspect_image(image_ref).await.is_ok() {
        return Ok(());
    }
    let options = CreateImageOptionsBuilder::default().from_image(image_ref).build();
    let mut stream = docker.create_image(Some(options), None, None);
    while let Some(progress) = stream.next().await {
        progress.with_context(|| format!("pulling {image_ref}"))?;
    }
    Ok(())
}

/// Environment seen by the experiment: the spec env plus the seed.
pub fn container_env(spec: &JobSpec) -> Vec<String> {
    spec.env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .chain([format!("REPLICATA_SEED={}", spec.seed), "PYTHONHASHSEED=0".to_string()])
        .collect()
}

pub fn host_config(spec: &JobSpec, inputs: &Path, out: &Path) -> HostConfig {
    HostConfig {
        binds: Some(vec![format!("{}:/inputs:ro", inputs.display()), format!("{}:/out:rw", out.display())]),
        network_mode: Some("none".into()),
        readonly_rootfs: Some(true),
        cap_drop: Some(vec!["ALL".into()]),
        security_opt: Some(vec!["no-new-privileges".into()]),
        memory: Some(i64::from(spec.mem_mb) * 1024 * 1024),
        nano_cpus: Some(i64::from(spec.cpus) * 1_000_000_000),
        pids_limit: Some(256),
        tmpfs: Some([("/tmp".to_string(), "rw,size=64m".to_string())].into()),
        ..Default::default()
    }
}

async fn logs(docker: &Docker, id: &str) -> String {
    let options = LogsOptionsBuilder::default().stdout(true).stderr(true).tail("200").build();
    let chunks: Vec<String> =
        docker.logs(id, Some(options)).filter_map(|c| async { c.ok().map(|c| c.to_string()) }).collect().await;

    chunks.concat()
}

async fn wait(docker: &Docker, id: &str, timeout: Duration) -> (i64, bool) {
    let waited = tokio::time::timeout(timeout, docker.wait_container(id, None).next()).await;
    match waited {
        Ok(Some(Ok(response))) => (response.status_code, false),
        // bollard reports non-zero exits as an error carrying the code.
        Ok(Some(Err(bollard::errors::Error::DockerContainerWaitError { code, .. }))) => (code, false),
        Ok(_) => (-1, false),
        Err(_) => {
            let kill = KillContainerOptionsBuilder::default().signal("SIGKILL").build();
            let _ = docker.kill_container(id, Some(kill)).await;
            (-1, true)
        }
    }
}

/// `nobody:nogroup`, used when the worker itself runs as root.
#[cfg(unix)]
const NOBODY: (u32, u32) = (65534, 65534);

/// Never run as root: use the owner of the output directory, so the experiment can write its
/// results (with every capability dropped, root could not write into a user-owned mount). When the
/// worker runs as root (servers, CI) that owner is root, so the directory goes to `nobody` instead.
/// Any failure aborts the run before anything is committed: an experiment that cannot write would
/// produce the hash of an empty output and cost the worker its bond.
#[cfg(unix)]
fn run_as(out: &Path) -> anyhow::Result<Option<String>> {
    use std::os::unix::fs::MetadataExt;

    let meta = std::fs::metadata(out).context("reading the output directory owner")?;
    if meta.uid() != 0 {
        return Ok(Some(format!("{}:{}", meta.uid(), meta.gid())));
    }
    std::os::unix::fs::chown(out, Some(NOBODY.0), Some(NOBODY.1)).context("handing /out to nobody")?;

    Ok(Some(format!("{}:{}", NOBODY.0, NOBODY.1)))
}

#[cfg(not(unix))]
fn run_as(_out: &Path) -> anyhow::Result<Option<String>> {
    Ok(None)
}

/// Runs the experiment; `out` must be an empty, writable directory.
pub async fn run(docker: &Docker, spec: &JobSpec, inputs: &Path, out: &Path) -> anyhow::Result<RunOutcome> {
    let image = spec.image_ref();
    ensure_image(docker, &image).await?;
    let body = ContainerCreateBody {
        image: Some(image),
        cmd: Some(spec.cmd.clone()),
        env: Some(container_env(spec)),
        network_disabled: Some(true),
        working_dir: Some("/out".into()),
        user: run_as(out)?,
        host_config: Some(host_config(spec, inputs, out)),
        ..Default::default()
    };
    let name = format!("replicata-{}", rand::random::<u64>());
    let created =
        docker.create_container(Some(CreateContainerOptionsBuilder::default().name(&name).build()), body).await?;
    docker.start_container(&created.id, None).await?;
    let (exit_code, timed_out) = wait(docker, &created.id, Duration::from_secs(spec.timeout_s.into())).await;
    let logs = logs(docker, &created.id).await;
    let remove = RemoveContainerOptionsBuilder::default().force(true).build();
    let _ = docker.remove_container(&created.id, Some(remove)).await;

    Ok(RunOutcome { exit_code, timed_out, logs })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn run_as_uses_the_output_owner_and_never_falls_back_to_the_image_user() {
        let dir = tempfile::tempdir().unwrap();
        let user = run_as(dir.path()).unwrap().unwrap();

        assert_ne!(user, "0:0", "never root");
        assert!(run_as(&dir.path().join("missing")).is_err(), "an unreadable /out must abort the run");
    }
}
