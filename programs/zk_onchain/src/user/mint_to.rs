use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Token, TokenAccount},
    token_interface::Mint,
};
use light_compressed_token::{cpi::accounts::MintToInstruction, program::LightCompressedToken};
use light_system_program::program::LightSystemProgram;
#[derive(Accounts)]
pub struct MintTokens<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub service_signer: Signer<'info>,

    #[account(
        seeds = [
            crate::state::MINT_AUTHORITY.as_bytes(),
        ],
        bump,
        constraint = mint.mint_authority.unwrap() == authority.key()
    )]
    pub authority: UncheckedAccount<'info>,

    pub cpi_authority_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_pool_pda: Account<'info, TokenAccount>,

    pub light_system_program: Program<'info, LightSystemProgram>,

    pub registered_program_pda: UncheckedAccount<'info>,

    pub noop_program: UncheckedAccount<'info>,

    pub account_compression_authority: UncheckedAccount<'info>,

    pub account_compression_program: Program<'info, AccountCompression>,

    #[account(mut)]
    pub merkle_tree: UncheckedAccount<'info>,

    #[account(mut)]
    pub sol_pool_pda: Option<AccountInfo<'info>>,

    #[account(
        mut,
        constraint = mint.mint_authority.is_some()
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,

    pub light_compressed_token: Program<'info, LightCompressedToken>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent_program: Sysvar<'info, Rent>,
}

pub fn mint_tokens<'info>(
    ctx: Context<MintTokens>,
    public_keys: Vec<Pubkey>,
    amounts: Vec<u64>,
    lamports: Option<u64>,
) -> Result<()> {
    let cpi_accounts = MintToInstruction {
        fee_payer: ctx.accounts.payer.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
        cpi_authority_pda: ctx.accounts.cpi_authority_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        token_pool_pda: ctx.accounts.token_pool_pda.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        light_system_program: ctx.accounts.light_system_program.to_account_info(),
        registered_program_pda: ctx.accounts.registered_program_pda.to_account_info(),
        noop_program: ctx.accounts.noop_program.to_account_info(),
        account_compression_authority: ctx.accounts.account_compression_authority.to_account_info(),
        account_compression_program: ctx.accounts.account_compression_program.to_account_info(),
        merkle_tree: ctx.accounts.merkle_tree.to_account_info(),
        self_program: ctx.accounts.light_compressed_token.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        sol_pool_pda: None,
    };

    let (_, bump_seed_payer) =
        Pubkey::find_program_address(&[ctx.accounts.payer.key().as_ref()], ctx.program_id);
    let payer_key = ctx.accounts.payer.key();

    let payer_bump = [bump_seed_payer];
    let authority_bump = [ctx.bumps.authority];

    let seeds = [
        &[payer_key.as_ref(), &payer_bump][..],
        &[crate::state::MINT_AUTHORITY.as_bytes(), &authority_bump][..],
    ];

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.light_compressed_token.to_account_info(),
        cpi_accounts,
        &seeds[..],
    );

    light_compressed_token::cpi::mint_to(cpi_context, public_keys, amounts, lamports)
}
