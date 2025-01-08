use crate::program::ZkOnchain;
use anchor_lang::prelude::*;
use light_compressed_token::{
    process_transfer::{
        CompressedTokenInstructionDataTransfer, InputTokenDataWithContext,
        PackedTokenTransferOutputData,
    },
    program::LightCompressedToken,
};
use light_sdk::{light_system_accounts, LightTraits};
use light_system_program::{invoke::processor::CompressedProof, sdk::CompressedCpiContext};

#[light_system_accounts]
#[derive(Accounts, LightTraits)]
pub struct TransferCompressedTokensWithPda<'info> {
    #[account(mut)]
    #[fee_payer]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub source_pda_owner: Signer<'info>,

    #[account(mut)]
    pub source_pda: AccountInfo<'info>,

    #[account(mut)]
    pub destination_pda: AccountInfo<'info>,

    pub compressed_token_program: Program<'info, LightCompressedToken>,

    pub compressed_token_cpi_authority_pda: AccountInfo<'info>,

    #[self_program]
    pub self_program: Program<'info, ZkOnchain>,

    #[cpi_context]
    #[account(mut)]
    pub cpi_context_account: AccountInfo<'info>,

    #[authority]
    #[account(mut)]
    pub cpi_authority_pda: AccountInfo<'info>,
}

pub fn transfer_compressed_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferCompressedTokensWithPda<'info>>,
    transfer_amount: u64,
    proof: CompressedProof,
    mint: Pubkey,
    input_token_data_with_context: Vec<InputTokenDataWithContext>,
    output_state_merkle_tree_account_indices: Vec<u8>,
    mut cpi_context: CompressedCpiContext,
) -> Result<()> {
    // Create the destination token data
    let destination_token_data = PackedTokenTransferOutputData {
        amount: transfer_amount,
        owner: ctx.accounts.destination_pda.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[0],
        tlv: None,
    };

    // Calculate remaining amount for source
    let source_remaining_amount = input_token_data_with_context[0].amount - transfer_amount;
    let change_token_data = PackedTokenTransferOutputData {
        amount: source_remaining_amount,
        owner: ctx.accounts.source_pda_owner.key(),
        lamports: None,
        merkle_tree_index: output_state_merkle_tree_account_indices[1],
        tlv: None,
    };

    let output_compressed_accounts = vec![change_token_data, destination_token_data];

    // Set CPI context flags
    cpi_context.set_context = true;

    // Create transfer instruction data
    let inputs_struct = CompressedTokenInstructionDataTransfer {
        proof: Some(proof),
        mint,
        delegated_transfer: None,
        input_token_data_with_context,
        output_compressed_accounts,
        is_compress: false,
        compress_or_decompress_amount: None,
        cpi_context: Some(cpi_context),
        lamports_change_account_merkle_tree_index: None,
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&inputs_struct, &mut inputs).unwrap();

    // Create CPI accounts
    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.signer.to_account_info(),
        authority: ctx.accounts.source_pda_owner.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx
            .accounts
            .compressed_token_cpi_authority_pda
            .to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        token_pool_pda: None,
        compress_or_decompress_token_account: None,
        token_program: None,
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    // Create CPI context
    let cpi_ctx = CpiContext::new(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
    )
    .with_remaining_accounts(ctx.remaining_accounts.to_vec());

    // Execute the transfer
    light_compressed_token::cpi::transfer(cpi_ctx, inputs)?;

    Ok(())
}
