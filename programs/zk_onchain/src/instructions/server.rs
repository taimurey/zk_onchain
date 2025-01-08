use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use light_sdk::compressed_account::LightAccount;
use light_sdk::merkle_context::PackedAddressMerkleContext;
use light_sdk::{light_account, light_accounts};

use crate::state::CustomError;
use crate::{ParamsCreateServer, ParamsTransferServerOwnership};

// Server Account Structure
#[light_account]
#[derive(Clone, Debug, Default)]
pub struct ServerAccount {
    #[truncate]
    pub authority: Pubkey,
    #[truncate]
    pub name: String,
    #[truncate]
    pub ticker: String,
    pub theme: Option<Pubkey>,
}

#[light_accounts]
#[instruction(name: String, ticker: String)]
pub struct CreateServer<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,
    #[self_program]
    pub self_program: Program<'info, crate::program::ZkOnchain>,
    /// CHECK: Checked in light-system-program.
    #[authority]
    pub cpi_signer: AccountInfo<'info>,
    #[light_account(init, seeds = [b"server", name.as_bytes(), ticker.as_bytes()])]
    pub server_account: LightAccount<ServerAccount>,
}

#[light_accounts]
pub struct TransferServerOwnership<'info> {
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
        seeds = [b"server", server_account.name.as_bytes(), server_account.ticker.as_bytes()],
        constraint = server_account.authority == signer.key() @ CustomError::Unauthorized
    )]
    pub server_account: LightAccount<ServerAccount>,
}
