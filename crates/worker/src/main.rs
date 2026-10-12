use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use replicata_client::Pubkey;
use replicata_core::net::format_usdc;
use replicata_worker::api::Coordinator;
use replicata_worker::chain::Chain;
use replicata_worker::job::Worker;
use replicata_worker::state::Stage;
use replicata_worker::{home, load_or_create_keypair, pay};
use solana_signer::Signer;

#[derive(Parser)]
#[command(name = "replicata", version, about = "Replicate computational experiments on Solana and get paid in USDC")]
struct Cli {
    /// Coordinator URL.
    #[arg(long, global = true, env = "REPLICATA_COORDINATOR", default_value = "http://127.0.0.1:8787")]
    coordinator: String,
    /// Keypair file (default: $REPLICATA_HOME/id.json).
    #[arg(long, global = true, env = "REPLICATA_KEYPAIR")]
    keypair: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExitAfter {
    Claimed,
    Executed,
    Committed,
    Uploaded,
}

#[derive(Subcommand)]
enum Command {
    /// Create the worker keypair and token account; print funding instructions.
    Init,
    /// Show address, balances and reputation.
    Status,
    /// Replicate experiments: one (`--once ID`) or every open one, forever.
    Run {
        #[arg(long)]
        once: Option<String>,
        /// Test only: tamper with the output (this worker will disagree and lose its bond).
        #[arg(long, hide = true)]
        chaos: bool,
        /// Test only: stop after a stage (simulates a crash; run again to resume).
        #[arg(long, hide = true, value_enum)]
        exit_after: Option<ExitAfter>,
    },
    /// Recompute the canonical hash of a result bundle.
    Verify {
        bundle: PathBuf,
        /// Fail unless the hash equals this value.
        #[arg(long)]
        expected: Option<String>,
    },
    /// Pay (x402) for a verified result and save it.
    Buy {
        id: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Publish an experiment as an agent: pay the fee over x402, sign and send the transaction.
    Publish {
        /// JSON body of POST /v1/experiments without `authority` (filled with this keypair).
        request: PathBuf,
    },
}

impl ExitAfter {
    fn stage(self) -> Stage {
        match self {
            Self::Claimed => Stage::Claimed,
            Self::Executed => Stage::Executed,
            Self::Committed => Stage::Committed,
            Self::Uploaded => Stage::Uploaded,
        }
    }
}

async fn connect(cli: &Cli) -> anyhow::Result<(Coordinator, Chain)> {
    let api = Coordinator::new(&cli.coordinator);
    let config = api.config().await?;
    let path = cli.keypair.clone().unwrap_or_else(|| home().join("id.json"));
    let (keypair, _) = load_or_create_keypair(&path)?;
    let chain = Chain::new(&config.rpc_url, Arc::new(keypair), config.mint.parse::<Pubkey>()?);

    Ok((api, chain))
}

async fn init(cli: &Cli) -> anyhow::Result<()> {
    let path = cli.keypair.clone().unwrap_or_else(|| home().join("id.json"));
    let (keypair, created) = load_or_create_keypair(&path)?;
    let config = Coordinator::new(&cli.coordinator).config().await?;
    println!("{} worker key {} ({})", if created { "created" } else { "existing" }, keypair.pubkey(), path.display());
    println!("cluster: {} — fund it with SOL ({}) and USDC ({})", config.cluster, config.faucets.sol, config.faucets.usdc);
    let chain = Chain::new(&config.rpc_url, Arc::new(keypair), config.mint.parse()?);
    match chain.ensure_token_account().await {
        Ok(()) => println!("USDC token account ready"),
        Err(_) => println!("add SOL, then run `replicata init` again to create the USDC token account"),
    }
    Ok(())
}

async fn status(cli: &Cli) -> anyhow::Result<()> {
    let (api, chain) = connect(cli).await?;
    let (lamports, tokens) = chain.balances().await?;
    let profile: serde_json::Value = reqwest::get(format!("{}/v1/workers/{}", api.base, chain.pubkey())).await?.json().await?;
    println!("worker  {}", chain.pubkey());
    println!("SOL     {:.4}", lamports as f64 / 1e9);
    println!("USDC    {}", format_usdc(tokens));
    println!("record  ok {} · bad {} · earned {} USDC", profile["ok"], profile["bad"], format_usdc(profile["earned"].as_u64().unwrap_or(0)));
    Ok(())
}

fn worker(api: Coordinator, chain: Chain, chaos: bool, exit_after: Option<ExitAfter>) -> Worker {
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());

    Worker { chain, api, http, home: home(), chaos, exit_after: exit_after.map(ExitAfter::stage) }
}

async fn run(cli: &Cli, once: Option<String>, chaos: bool, exit_after: Option<ExitAfter>) -> anyhow::Result<()> {
    let (api, chain) = connect(cli).await?;
    let me = chain.pubkey().to_string();
    let worker = worker(api.clone(), chain, chaos, exit_after);
    if let Some(id) = once {
        let stage = worker.process(&id).await?;
        println!("done: {stage:?}");
        return Ok(());
    }
    println!("worker {me} polling {} …", api.base);
    loop {
        let open = api.experiments("status=open&phase=claiming").await.unwrap_or_default();
        for exp in open.iter().filter(|e| e.claims_count < e.max_claims && e.claim_of(&me).is_none()) {
            if let Err(err) = worker.process(&exp.id).await {
                eprintln!("[{}] {err:#}", exp.id);
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn buy(cli: &Cli, id: &str, output: Option<PathBuf>) -> anyhow::Result<()> {
    let (api, chain) = connect(cli).await?;
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());
    let bought = pay::buy(&http, &api.base, id).await?;
    let path = output.unwrap_or_else(|| PathBuf::from(format!("{id}.tar.gz")));
    std::fs::write(&path, &bought.bundle)?;
    println!("verified result {} saved to {}", bought.result_hash, path.display());
    if let Some(settlement) = bought.settlement {
        println!("payment receipt (PAYMENT-RESPONSE): {settlement}");
    }
    Ok(())
}

async fn publish(cli: &Cli, request: PathBuf) -> anyhow::Result<()> {
    let (api, chain) = connect(cli).await?;
    let mut body: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(request)?)?;
    body["authority"] = chain.pubkey().to_string().into();
    let http = pay::paying_client(chain.keypair.clone(), chain.rpc.clone());
    let (experiment, signature) = pay::publish(&http, &chain, &api.base, body).await?;
    println!("published experiment {experiment} (tx {signature})");
    Ok(())
}

fn verify(bundle: PathBuf, expected: Option<String>) -> anyhow::Result<()> {
    let hash = replicata_core::to_hex(&replicata_core::bundle::hash_bundle(&std::fs::read(bundle)?)?);
    println!("{hash}");
    if let Some(expected) = expected {
        anyhow::ensure!(hash == expected.to_lowercase(), "hash mismatch: expected {expected}");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Init => init(&cli).await,
        Command::Status => status(&cli).await,
        Command::Run { once, chaos, exit_after } => run(&cli, once.clone(), *chaos, *exit_after).await,
        Command::Verify { bundle, expected } => verify(bundle.clone(), expected.clone()),
        Command::Buy { id, output } => buy(&cli, id, output.clone()).await,
        Command::Publish { request } => publish(&cli, request.clone()).await,
    }
}
