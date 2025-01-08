use anchor_lang::prelude::*;
use anchor_spl::token::Token;

#[derive(Accounts)]
pub struct UserVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: soda authority account
    #[account(
        seeds = [
            crate::state::SODA_AUTHORITY.as_bytes(),
            &creator.key().to_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        space = 0,
        seeds = [
            crate::state::USER_VAULT.as_bytes(),
            &creator.key().to_bytes(),
        ],
        bump,
    )]
    pub user_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ServerVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: soda authority account
    #[account(
        seeds = [
            crate::state::SODA_AUTHORITY.as_bytes(),
            &creator.key().to_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        space = 0,
        seeds = [
            crate::state::SERVER_VAULT.as_bytes(),
            &creator.key().to_bytes(),
        ],
        bump,
    )]
    pub server_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AirdropVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: soda authority account
    #[account(
        seeds = [
            crate::state::SODA_AUTHORITY.as_bytes(),
            &creator.key().to_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: airdrop vault PDA
    #[account(
    init,
    payer = creator,
    space = 8 + 1,
    seeds = [
        crate::state::AIRDROP_VAULT.as_bytes(),
        creator.key().as_ref(),
    ],
    bump,
)]
    pub airdrop_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct EscrowVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(mut)]
    pub creator: Signer<'info>,

    /// CHECK: soda authority account
    #[account(
        seeds = [
            crate::state::SODA_AUTHORITY.as_bytes(),
            &creator.key().to_bytes(),
        ],
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: airdrop vault PDA
    #[account(
    init,
    payer = creator,
    space = 8 + 1,
    seeds = [
        crate::state::ESCROW_VAULT.as_bytes(),
        creator.key().as_ref(),
    ],
    bump,
)]
    pub escrow_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}
