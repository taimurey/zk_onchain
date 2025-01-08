use crate::state::CustomError;
use crate::{
 ParamsCreateUser, ParamsCreateUserHandle, ParamsTransferUserHandle,
    ParamsUpdateUserProfile,
};
use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_hasher::bytes::AsByteVec;
use light_sdk::compressed_account::LightAccount;
use light_sdk::merkle_context::PackedAddressMerkleContext;
use light_sdk::{light_account, light_accounts};

#[derive(Accounts)]
pub struct Initialize {}

#[light_accounts]
#[instruction(username: String)]
pub struct CreateUser<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,

    #[account(mut)]
    #[fee_payer]
    pub creator: Signer<'info>,

    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,

    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(
        init,
        seeds = [    
        crate::state::USER_VAULT.as_bytes(),
        &creator.key().to_bytes(),
        ]
    )]
    pub user_vault: LightAccount<UserAccount>,
    
    // #[account(
    //     init,
    //     payer = signer,
    //     space = 8+1,
    //     seeds = [
    //         crate::USER_VAULT.as_bytes(),
    //         signer.key().as_ref(),
    //     ],
    //     bump
    // )]
    // pub user_vault: Account<'info, UserVault>,
}



#[light_account]
#[derive(Clone, Debug, Default)]
pub struct UserAccount {
    #[truncate]
    pub authority: Pubkey,
    #[truncate]
    pub username: String,
    pub handle: Option<Pubkey>,
    pub profile_effect: Option<Pubkey>,
    pub theme: Option<Pubkey>,
    pub vault: Pubkey,
}

#[light_account]
#[derive(Clone, Debug, Default)]
pub struct AssetAccount {
    #[truncate]
    pub owner: Pubkey,
    pub asset_type: AssetType,
    #[truncate]
    pub data: String,
}

#[derive(Clone, Debug, Eq, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum AssetType {
    UserHandle,
    ProfileEffect,
    Theme,
    Other,
}

impl anchor_lang::IdlBuild for AssetType {}

impl AsByteVec for AssetType {
    fn as_byte_vec(&self) -> Vec<Vec<u8>> {
        vec![vec![self.clone() as u8]]
    }
}

impl Default for AssetType {
    fn default() -> Self {
        Self::Other
    }
}


#[light_accounts]
pub struct UpdateUserProfile<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    #[light_account(
        mut,
        seeds = [b"user", user_account.username.as_bytes()],
        constraint = user_account.authority == signer.key() @ CustomError::Unauthorized
    )]
    pub user_account: LightAccount<UserAccount>,
}

#[light_accounts]
#[instruction(handle: String)]
pub struct CreateUserHandle<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    #[light_account(
        mut,
        seeds = [b"user", user_account.username.as_bytes()],
        constraint = user_account.authority == signer.key() @ CustomError::Unauthorized
    )]
    pub user_account: LightAccount<UserAccount>,
    #[light_account(init, seeds = [b"handle", handle.as_bytes()])]
    pub handle_asset: LightAccount<AssetAccount>,
}

#[light_accounts]
pub struct TransferUserHandle<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,

    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,

    #[light_account(
        mut,
        seeds = [b"user", from_user_account.username.as_bytes()],
        constraint = from_user_account.authority == signer.key() @ CustomError::Unauthorized
    )]
    pub from_user_account: LightAccount<UserAccount>,

    #[light_account(mut, seeds = [b"user", to_user_account.username.as_bytes()])]
    pub to_user_account: LightAccount<UserAccount>,

    #[light_account(
        mut,
        seeds = [b"handle", handle_asset.data.as_bytes()],
      //  constraint = handle_asset.owner == from_user_account. @ CustomError::Unauthorized
    )]
    pub handle_asset: LightAccount<AssetAccount>,
}
