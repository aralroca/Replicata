//! Downloads experiment inputs (paying x402 when a URL asks for it) and checks their hashes.

use std::path::Path;

use replicata_core::Input;
use reqwest_middleware::ClientWithMiddleware;
use sha2::{Digest, Sha256};

use crate::pay::check;

pub async fn fetch_all(http: &ClientWithMiddleware, inputs: &[Input], dir: &Path) -> anyhow::Result<()> {
    for input in inputs {
        let bytes = check(http.get(&input.url).send().await?).await?.bytes().await?;
        verify(input, &bytes)?;
        let path = dir.join(&input.path);
        std::fs::create_dir_all(path.parent().unwrap_or(dir))?;
        std::fs::write(path, bytes)?;
    }
    Ok(())
}

pub fn verify(input: &Input, bytes: &[u8]) -> anyhow::Result<()> {
    let actual = hex::encode(Sha256::digest(bytes));
    anyhow::ensure!(actual == input.sha256.to_lowercase(), "input {} has sha256 {actual}, expected {}", input.path, input.sha256);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn input(url: String, content: &[u8], path: &str) -> Input {
        Input { url, sha256: hex::encode(Sha256::digest(content)), path: path.into() }
    }

    #[tokio::test]
    async fn downloads_and_verifies_inputs() {
        let server = MockServer::start().await;
        Mock::given(path("/data.csv")).respond_with(ResponseTemplate::new(200).set_body_bytes(b"1,2\n")).mount(&server).await;
        let dir = tempfile::tempdir().unwrap();
        let http = reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build();

        fetch_all(&http, &[input(format!("{}/data.csv", server.uri()), b"1,2\n", "a/data.csv")], dir.path()).await.unwrap();
        assert_eq!(std::fs::read(dir.path().join("a/data.csv")).unwrap(), b"1,2\n");

        let tampered = input(format!("{}/data.csv", server.uri()), b"other", "x.csv");
        assert!(fetch_all(&http, &[tampered], dir.path()).await.unwrap_err().to_string().contains("sha256"));
    }

    #[tokio::test]
    async fn unpaid_inputs_fail_with_a_clear_error() {
        let server = MockServer::start().await;
        Mock::given(path("/paid")).respond_with(ResponseTemplate::new(402).set_body_string("{\"error\":\"payment_required\"}")).mount(&server).await;
        let http = reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build();
        let dir = tempfile::tempdir().unwrap();

        let err = fetch_all(&http, &[input(format!("{}/paid", server.uri()), b"", "p")], dir.path()).await.unwrap_err();
        assert!(err.to_string().contains("402"));
    }
}
