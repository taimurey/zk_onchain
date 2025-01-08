use crate::{state::SERVER_VAULT, ParamsInitializeServerVault, ParamsUpdateServerVaultAuthority};
use anchor_lang::prelude::*;
use light_sdk::{
    compressed_account::LightAccount, light_account, light_accounts,
    merkle_context::PackedAddressMerkleContext,
};

use crate::{state::VAULT_CONFIG_SEED, VaultConfigState};

use super::VaultType;

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct ServerVaultParams {
    pub server_id: String,
    pub server_name: String,
}

#[light_account]
#[derive(Clone, Debug, Default)]
pub struct ServerVaultState {
    pub server_id: String,
    pub server_name: String,
    #[truncate]
    pub current_authority: Pubkey,
    pub vault_type: VaultType,
    pub created_at: i64,
    pub modified_at: i64,
}

#[light_accounts]
pub struct InitializeServerVault<'info> {
    #[account(mut)]
    #[fee_payer]
    pub payer: Signer<'info>,

    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,

    /// CHECK: Checked in light-system-program.
    pub service_signer: Signer<'info>,

    pub current_authority: Signer<'info>,

    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(
        init,
        seeds = [
            SERVER_VAULT.as_bytes(),
            current_authority.key().as_ref()
        ],
    )]
    pub server_vault: LightAccount<ServerVaultState>,

    #[account(
        seeds = [VAULT_CONFIG_SEED.as_bytes(), config_authority.key().as_ref()],
        bump
    )]
    pub config: AccountLoader<'info, VaultConfigState>,

    /// CHECK: Config authority pubkey used for PDA derivation
    pub config_authority: AccountInfo<'info>,
}

#[light_accounts]
pub struct UpdateServerVaultAuthority<'info> {
    #[account(mut)]
    #[fee_payer]
    pub payer: Signer<'info>,

    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,

    /// CHECK: Checked in light-system-program.
    pub service_signer: Signer<'info>,

    /// CHECK:
    pub current_authority: Signer<'info>,

    pub new_authority: Signer<'info>,

    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(
        mut,
        seeds = [
            SERVER_VAULT.as_bytes(),
            current_authority.key().as_ref()
        ]
    )]
    pub server_vault: LightAccount<ServerVaultState>,
}
