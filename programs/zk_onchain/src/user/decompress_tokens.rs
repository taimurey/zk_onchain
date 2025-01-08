use crate::state::CustomError;
use account_compression::program::AccountCompression;
use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use light_compressed_token::cpi::accounts::TransferInstruction;
use light_compressed_token::process_transfer::CompressedTokenInstructionDataTransfer;
use light_compressed_token::program::LightCompressedToken;
use light_system_program::program::LightSystemProgram;

#[derive(Accounts)]
pub struct DecompressTokens<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub service_signer: Signer<'info>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub cpi_authority_pda: UncheckedAccount<'info>,

    #[account(mut)]
    pub decompress_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub token_pool_pda: UncheckedAccount<'info>,

    /// CHECK:
    pub registered_program_pda: UncheckedAccount<'info>,

    /// CHECK:
    pub noop_program: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    pub account_compression_authority: UncheckedAccount<'info>,

    pub light_compressed_token: Program<'info, LightCompressedToken>,
    pub account_compression_program: Program<'info, AccountCompression>,
    pub light_system_program: Program<'info, LightSystemProgram>,
    pub system_program: Program<'info, System>,
}

pub fn decompress_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, DecompressTokens<'info>>,
    compressed_params: Vec<u8>,
) -> Result<()> {
    let inputs: CompressedTokenInstructionDataTransfer =
        CompressedTokenInstructionDataTransfer::deserialize(&mut compressed_params.as_slice())
            .map_err(|_| {
                msg!("Failed to deserialize compressed_params.");
                CustomError::InvalidCompressedParams
            })?;

    light_compressed_token::cpi::transfer(
        ctx.accounts
            .set_uncompress_ctx(
                ctx.accounts.token_program.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                Some(ctx.accounts.decompress_account.to_account_info()),
            )
            .with_remaining_accounts(ctx.remaining_accounts.to_vec()),
        inputs.try_to_vec()?,
    )
}

impl<'info> DecompressTokens<'info> {
    pub fn set_uncompress_ctx<'a>(
        &self,
        token_program: AccountInfo<'info>,
        authority: AccountInfo<'info>,
        compress_or_decompress_token_account: Option<AccountInfo<'info>>,
    ) -> CpiContext<'a, 'a, 'a, 'info, TransferInstruction<'info>> {
        let cpi_accounts = light_compressed_token::cpi::accounts::TransferInstruction {
            fee_payer: self.payer.to_account_info(),
            authority: authority.to_account_info(),
            registered_program_pda: self.registered_program_pda.to_account_info(),
            noop_program: self.noop_program.to_account_info(),
            account_compression_authority: self.account_compression_authority.to_account_info(),
            account_compression_program: self.account_compression_program.to_account_info(),
            self_program: self.light_compressed_token.to_account_info(),
            cpi_authority_pda: self.cpi_authority_pda.to_account_info(),
            light_system_program: self.light_system_program.to_account_info(),
            token_pool_pda: Some(self.token_pool_pda.to_account_info()),
            compress_or_decompress_token_account,
            token_program: Some(token_program.to_account_info()),
            system_program: self.system_program.to_account_info(),
        };

        CpiContext::new(self.light_compressed_token.to_account_info(), cpi_accounts)
    }
}
