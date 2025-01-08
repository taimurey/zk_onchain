use std::str::FromStr;

use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use light_compressed_token::program::LightCompressedToken;
use light_sdk::merkle_context::AddressMerkleContext;
use light_system_program::cpi::accounts::InvokeCpiInstruction;
use light_utils::{hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be};

use crate::state::USER_VAULT;

#[derive(Accounts)]
pub struct TransferCompressedTokensWallet<'info> {
    /// Fee payer needs to be mutable to pay rollover and protocol fees.
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub service_signer: Signer<'info>,

    pub current_authority: Signer<'info>,

    #[account(mut)]
    pub user_vault: AccountInfo<'info>,

    pub self_program: Program<'info, crate::program::ZkOnchain>,

    pub registered_program_pda: AccountInfo<'info>,

    pub noop_program: UncheckedAccount<'info>,

    pub account_compression_authority: UncheckedAccount<'info>,

    pub account_compression_program: Program<'info, AccountCompression>,

    pub system_program: Program<'info, System>,

    pub light_compressed_token: Program<'info, LightCompressedToken>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub rent_program: Sysvar<'info, Rent>,
}

pub fn transfer_compressed_tokens_wallet<'info>(
    ctx: Context<TransferCompressedTokensWallet>,
    transfer_inputs: Vec<u8>,
) -> Result<()> {
    let cpi_accounts = InvokeCpiInstruction {
        fee_payer: ctx.accounts.user_vault.to_account_info(),
        authority: ctx.accounts.user_vault.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        sol_pool_pda: None,
        decompression_recipient: None,
        invoking_program: ctx.accounts.self_program.to_account_info(),
        cpi_context_account: None,
    };

    let (user_vault, bump) = derive_user_vault_with_bump(ctx.accounts.current_authority.key());
    let binding = ctx.accounts.current_authority.key();
    msg!("{}", Pubkey::from(user_vault));
    let bump_bytes = [bump];
    let seeds = &[USER_VAULT.as_bytes(), binding.as_ref(), &bump_bytes];
    let signer_seeds = &[&seeds[..]];

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.light_compressed_token.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    light_system_program::cpi::invoke_cpi(cpi_context, transfer_inputs)
}

pub fn derive_user_vault_with_bump(authority: Pubkey) -> ([u8; 32], u8) {
    let address_merkle_tree_queue_pubkey =
        Pubkey::from_str("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F").unwrap();

    let address_merkle_tree_pubkey =
        Pubkey::from_str("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2").unwrap();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey: address_merkle_tree_queue_pubkey,
    };
    let address_seed =
        derive_address_seed(&[USER_VAULT.as_bytes(), authority.as_ref()], &crate::ID);

    let merkle_tree_pubkey = address_merkle_context.address_merkle_tree_pubkey.to_bytes();
    let final_input = [merkle_tree_pubkey, address_seed].concat();
    let (final_address, bump) = hash_to_bn254_field_size_be(final_input.as_slice()).unwrap();

    (final_address, bump)
}

pub fn derive_address_seed(seeds: &[&[u8]], program_id: &Pubkey) -> [u8; 32] {
    let mut inputs = Vec::with_capacity(seeds.len() + 1);
    let program_id = program_id.to_bytes();
    inputs.push(program_id.as_slice());
    inputs.extend(seeds);
    hashv_to_bn254_field_size_be(inputs.as_slice())
}
