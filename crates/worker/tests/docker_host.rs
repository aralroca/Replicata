//! A remote Docker daemon (`DOCKER_HOST=tcp://…`, e.g. a CI Docker-in-Docker service) must be
//! accepted; only local sockets used to be. Own test binary: it sets a process-wide env var.

#[test]
fn docker_host_accepts_tcp_daemons() {
    std::env::set_var("DOCKER_HOST", "tcp://127.0.0.1:2375");

    assert!(replicata_worker::sandbox::docker().is_ok(), "tcp:// DOCKER_HOST rejected");
}
