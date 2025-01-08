use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use light_compressed_token::{cpi::accounts::CreateTokenPoolInstruction, program::LightCompressedToken};
use mpl_token_metadata::{instructions::{CreateMetadataAccountV3,CreateMetadataAccountV3InstructionArgs}, types::DataV2};

pub const COMPRESSED_MINT_SEED: &str  = "compressed_mint";

#[derive(Accounts)]
#[instruction( 
    name: String,
    symbol: String,
    decimals: u8,
    uri: String,
    nonce: u16,
)]
pub struct CreateCompressedMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub service_signer: Signer<'info>,

    #[account(
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

    #[account(
        init,
        seeds = [
            COMPRESSED_MINT_SEED.as_bytes(),
            payer.key().as_ref(),
            &nonce.to_be_bytes()
        ],
        bump,
        mint::decimals = decimals,
        mint::authority = authority,
        payer = payer,
        mint::token_program = token_program,
    )]
    pub compressed_mint: Box<InterfaceAccount<'info, Mint>>,
    

    #[account(mut)]
    pub metadata_account: UncheckedAccount<'info>,

    pub compressed_token_program: Program<'info, LightCompressedToken>,

    pub token_program: Interface<'info, TokenInterface>,

    // Program to create system account
    pub system_program: Program<'info, System>,

    pub rent_program: Sysvar<'info, Rent>,

    #[account(executable)]
    pub mpl_token_metadata: AccountInfo<'info>
}

impl<'info> CreateCompressedMint<'info> {
    pub fn set_token_pool_ctx<'a>(
        &self,
        mint: AccountInfo<'info>,
        token_pool: AccountInfo<'info>,
    ) -> CpiContext<'a, 'a, 'a, 'info, CreateTokenPoolInstruction<'info>> {
        let cpi_accounts = CreateTokenPoolInstruction {
            fee_payer: self.payer.to_account_info(),
            mint: mint.to_account_info(),
            system_program: self.system_program.to_account_info(),
            token_program: self.token_program.to_account_info(),
            token_pool_pda: token_pool.to_account_info(),
            cpi_authority_pda: self.cpi_authority_pda.to_account_info(),
        };

        CpiContext::new(
            self.compressed_token_program.to_account_info(),
            cpi_accounts,
        )
    }
}


pub fn create_compressed_mint<'info>(
    ctx: Context<CreateCompressedMint>,
    name: String,
    symbol: String,
    _decimals: u8,
    uri: String,
    _nonce: u16,
) -> Result<()> {
    let (_, bump_seed) =
        Pubkey::find_program_address(&[ctx.accounts.payer.key().as_ref()], ctx.program_id);

    light_compressed_token::cpi::create_token_pool(
        ctx.accounts
            .set_token_pool_ctx(
                ctx.accounts.compressed_mint.to_account_info(),
                ctx.accounts.token_pool_pda.to_account_info(),
            )
            .with_signer(&[&[ctx.accounts.payer.key().as_ref(), &[bump_seed]]]),
    )?;

    let args = CreateMetadataAccountV3InstructionArgs {
        data: DataV2 {
            name,
            symbol,
            uri,
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        },
        is_mutable: false,
        collection_details: None,
    };
    
    let accounts = CreateMetadataAccountV3 {
        metadata: ctx.accounts.metadata_account.key(),
        mint: ctx.accounts.compressed_mint.key(),
        mint_authority: ctx.accounts.authority.key(),
        payer: ctx.accounts.payer.key(),
        update_authority: (ctx.accounts.authority.key(), false),
        system_program: ctx.accounts.system_program.key(),
        rent: None,
    };
    
    let create_metadata_ix = accounts.instruction(args);
    
    solana_program::program::invoke_signed(
        &create_metadata_ix,
        &[
            ctx.accounts.metadata_account.to_account_info(),
            ctx.accounts.compressed_mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.payer.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
        &[&[crate::state::MINT_AUTHORITY.as_bytes(), &[ctx.bumps.authority]]]
    )?;

    Ok(())
}
