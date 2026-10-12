use anchor_lang::prelude::*;
use anchor_spl::token_interface::{transfer_checked, Mint, TokenAccount, TransferChecked};

use crate::constants::EXPERIMENT_SEED;
use crate::state::Experiment;

/// Accounts every vault payout needs.
pub struct Vault<'a, 'info> {
    pub experiment: &'a Account<'info, Experiment>,
    pub vault: &'a InterfaceAccount<'info, TokenAccount>,
    pub mint: &'a InterfaceAccount<'info, Mint>,
    pub token_program: &'a AccountInfo<'info>,
}

impl<'info> Vault<'_, 'info> {
    /// Transfers `amount` from the experiment vault, signed by the experiment PDA.
    pub fn pay(&self, to: &AccountInfo<'info>, amount: u64) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
        let nonce = self.experiment.nonce.to_le_bytes();
        let bump = [self.experiment.bump];
        let seeds: &[&[u8]] = &[EXPERIMENT_SEED, self.experiment.authority.as_ref(), &nonce, &bump];
        let accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint.to_account_info(),
            to: to.clone(),
            authority: self.experiment.to_account_info(),
        };

        transfer_checked(
            CpiContext::new_with_signer(self.token_program.key(), accounts, &[seeds]),
            amount,
            self.mint.decimals,
        )
    }
}

/// Transfers `amount` from a user-owned token account (the user signs the transaction).
pub fn pay_into_vault<'info>(
    token_program: &AccountInfo<'info>,
    from: &InterfaceAccount<'info, TokenAccount>,
    vault: &InterfaceAccount<'info, TokenAccount>,
    mint: &InterfaceAccount<'info, Mint>,
    authority: &Signer<'info>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let accounts = TransferChecked {
        from: from.to_account_info(),
        mint: mint.to_account_info(),
        to: vault.to_account_info(),
        authority: authority.to_account_info(),
    };

    transfer_checked(CpiContext::new(token_program.key(), accounts), amount, mint.decimals)
}

/// Loads a token account passed through `remaining_accounts`, checking owner and mint.
pub fn load_token_account<'info>(
    info: &'info AccountInfo<'info>,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<InterfaceAccount<'info, TokenAccount>> {
    let account = InterfaceAccount::<TokenAccount>::try_from(info)?;

    require_keys_eq!(account.owner, *owner, crate::error::ReplicataError::InvalidRemainingAccounts);
    require_keys_eq!(account.mint, *mint, crate::error::ReplicataError::InvalidRemainingAccounts);
    Ok(account)
}
