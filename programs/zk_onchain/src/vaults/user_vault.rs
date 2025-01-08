use crate::{ParamsInitializeUserVault, ParamsUpdateUserVaultAuthority};
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::{
    compressed_account::LightAccount, light_account, light_accounts,
    merkle_context::PackedAddressMerkleContext,
};

use crate::{
    state::{USER_VAULT, VAULT_CONFIG_SEED},
    VaultConfigState,
};

#[derive(Clone, Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum VaultType {
    User,
    Server,
    Other,
}

impl anchor_lang::IdlBuild for VaultType {}

impl AsByteVec for VaultType {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        vec![vec![self.clone() as u8]]
    }
}

impl Default for VaultType {
    fn default() -> Self {
        Self::Other
    }
}

#[light_account]
#[derive(Clone, Debug, Default)]
pub struct UserVaultState {
    #[truncate]
    pub current_authority: Pubkey,
    pub vault_type: VaultType,
    pub modified_at: i64,
}

#[light_accounts]
pub struct InitializeUserVault<'info> {
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
            USER_VAULT.as_bytes(),
            current_authority.key().as_ref()
        ],
    )]
    pub user_vault: LightAccount<UserVaultState>,

    #[account(
        seeds = [VAULT_CONFIG_SEED.as_bytes(), config_authority.key().as_ref()],
        bump
    )]
    pub config: AccountLoader<'info, VaultConfigState>,

    /// CHECK: Config authority pubkey used for PDA derivation
    pub config_authority: AccountInfo<'info>,
}

#[light_accounts]
pub struct UpdateUserVaultAuthority<'info> {
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
            USER_VAULT.as_bytes(),
            current_authority.key().as_ref()
        ]
    )]
    pub user_vault: LightAccount<UserVaultState>,
}
