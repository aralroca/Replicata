//! Cluster constants shared by coordinator, worker and tests.

/// Replicata program id (same on localnet and devnet; keypair kept outside the repo).
pub const PROGRAM_ID: &str = "G9Ssnp8rjg7gpi4Sm3EZLoC73z9pdXd9etJzUDH39Wj9";

/// Circle's USDC on devnet (classic SPL Token, 6 decimals).
pub const USDC_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
pub const USDC_DECIMALS: u8 = 6;

/// CAIP-2 id of Solana devnet used by x402 v2.
pub const DEVNET_CAIP2: &str = "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1";

pub const DEVNET_RPC: &str = "https://api.devnet.solana.com";
pub const LOCALNET_RPC: &str = "http://127.0.0.1:8899";

pub const SOL_FAUCET_URL: &str = "https://faucet.solana.com";
pub const USDC_FAUCET_URL: &str = "https://faucet.circle.com";

/// PDA seeds (mirrors `programs/replicata/src/constants.rs`).
pub const EXPERIMENT_SEED: &[u8] = b"experiment";
pub const CLAIM_SEED: &[u8] = b"claim";
pub const WORKER_SEED: &[u8] = b"worker";

/// Formats base units of a 6-decimals token as a human amount (`1500000` → `"1.5"`).
pub fn format_usdc(amount: u64) -> String {
    let whole = amount / 1_000_000;
    let frac = amount % 1_000_000;
    if frac == 0 {
        return whole.to_string();
    }
    let frac = format!("{frac:06}");

    format!("{whole}.{}", frac.trim_end_matches('0'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_usdc() {
        assert_eq!(format_usdc(0), "0");
        assert_eq!(format_usdc(1_500_000), "1.5");
        assert_eq!(format_usdc(10_000), "0.01");
        assert_eq!(format_usdc(3), "0.000003");
    }
}
