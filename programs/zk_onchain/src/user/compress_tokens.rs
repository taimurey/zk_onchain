use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;
use light_compressed_token::program::LightCompressedToken;
use light_system_program::program::LightSystemProgram;

#[derive(Accounts)]
pub struct CompressTokens<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub service_signer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            crate::state::MINT_AUTHORITY.as_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub cpi_authority_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_pool_pda: UncheckedAccount<'info>,

    /// CHECK:
    pub registered_program_pda: UncheckedAccount<'info>,

    /// CHECK:
    pub noop_program: UncheckedAccount<'info>,

    pub compressed_token_program: Program<'info, LightCompressedToken>,

    pub compress_token_account: UncheckedAccount<'info>,

    pub account_compression_authority: UncheckedAccount<'info>,

    pub account_compression_program: Program<'info, AccountCompression>,

    pub light_system_program: Program<'info, LightSystemProgram>,

    pub token_program: Interface<'info, TokenInterface>,

    // Program to create system account
    pub system_program: Program<'info, System>,
}

pub fn compress_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, CompressTokens<'info>>,
    inputs: Vec<u8>,
) -> Result<()> {
    let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
        fee_payer: ctx.accounts.payer.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        self_program: ctx.accounts.compressed_token_program.to_account_info(),
        cpi_authority_pda: ctx.accounts.cpi_authority_pda.to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        token_pool_pda: Some(ctx.accounts.token_pool_pda.to_account_info()),
        compress_or_decompress_token_account: Some(
            ctx.accounts.compress_token_account.to_account_info(),
        ),
        token_program: Some(ctx.accounts.token_program.to_account_info()),
        system_program: ctx.accounts.system_program.to_account_info(),
    };

    let binding: &[&[&[u8]]] = &[&[
        crate::state::MINT_AUTHORITY.as_bytes(),
        &[ctx.bumps.authority],
    ]];

    let context = CpiContext::new_with_signer(
        ctx.accounts.compressed_token_program.to_account_info(),
        cpi_accounts,
        binding,
    );

    light_compressed_token::cpi::transfer(
        context.with_remaining_accounts(ctx.remaining_accounts.to_vec()),
        inputs.try_to_vec()?,
    )
}
