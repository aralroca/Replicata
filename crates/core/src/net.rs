//! Cluster constants shared by coordinator, worker and tests. The program id and PDA seeds
//! live in the program itself (re-exported by `replicata-client`).

/// Circle's USDC on devnet (classic SPL Token, 6 decimals).
pub const USDC_DEVNET: &str = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
pub const USDC_DECIMALS: u8 = 6;
const USDC_UNIT: u64 = 10u64.pow(USDC_DECIMALS as u32);

pub const DEVNET_RPC: &str = "https://api.devnet.solana.com";
pub const LOCALNET_RPC: &str = "http://127.0.0.1:8899";

pub const SOL_FAUCET_URL: &str = "https://faucet.solana.com";
pub const USDC_FAUCET_URL: &str = "https://faucet.circle.com";

/// Formats base units of a 6-decimals token as a human amount (`1500000` → `"1.5"`).
pub fn format_usdc(amount: u64) -> String {
    let whole = amount / USDC_UNIT;
    let frac = amount % USDC_UNIT;
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
