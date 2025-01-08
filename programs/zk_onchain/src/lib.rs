pub mod compressed_transfers;
pub mod instructions;
pub mod marketplace;
pub mod state;
pub mod user;
pub mod utils;
pub mod vaults;

use crate::compressed_transfers::*;
use crate::instructions::*;
use crate::user::*;
use crate::vaults::*;
use anchor_lang::prelude::*;
use borsh::BorshDeserialize;
use light_sdk::light_program;
pub use marketplace::*;

use light_compressed_token::process_transfer::InputTokenDataWithContext;
use light_system_program::invoke::processor::CompressedProof;
use light_system_program::sdk::CompressedCpiContext;

declare_id!("6b51XxnGuCQA3t7sZiHE4LrGdAayb79Vsh2Bkkz6gwqM");

#[light_program]
#[program]
pub mod zk_onchain {

    use state::CustomError;

    use super::*;

    pub fn initialize_vault_config(ctx: Context<InitializeConfig>) -> Result<()> {
        vaults::initialize_vault_config(ctx)
    }

    pub fn update_vault_authority(ctx: Context<UpdateVaultAuthority>) -> Result<()> {
        vaults::update_vault_authority(ctx)
    }

    pub fn manage_service_signer(
        ctx: Context<ManageServiceSigner>,
        operation: ServiceSignerOperation,
    ) -> Result<()> {
        vaults::manage_service_signer(ctx, operation)
    }

    pub fn initialize_user_vault<'info>(
        ctx: LightContext<'_, '_, '_, 'info, InitializeUserVault<'info>>,
    ) -> Result<()> {
        let clock = Clock::get()?;

        ctx.light_accounts.user_vault.current_authority = ctx.accounts.current_authority.key();
        ctx.light_accounts.user_vault.vault_type = VaultType::User;
        ctx.light_accounts.user_vault.modified_at = clock.unix_timestamp;

        Ok(())
    }

    pub fn update_user_vault_authority<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateUserVaultAuthority<'info>>,
    ) -> Result<()> {
        if ctx.accounts.current_authority.key() != ctx.light_accounts.user_vault.current_authority {
            return Ok(());
        }

        let clock = Clock::get()?;
        ctx.light_accounts.user_vault.current_authority = ctx.accounts.new_authority.key();
        ctx.light_accounts.user_vault.modified_at = clock.unix_timestamp;

        Ok(())
    }

    /// Initialize Server Vault
    //* ServerVaultParams: server_name, server_id
    pub fn initialize_server_vault<'info>(
        ctx: LightContext<'_, '_, '_, 'info, InitializeServerVault<'info>>,
        params: ServerVaultParams,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let current_timestamp = clock.unix_timestamp;

        let authority = ctx.accounts.current_authority.key();

        let server_vault = &mut ctx.light_accounts.server_vault;

        server_vault.current_authority = authority;
        server_vault.server_id = params.server_id;
        server_vault.vault_type = VaultType::Server;
        server_vault.server_name = params.server_name;
        server_vault.created_at = current_timestamp;
        server_vault.modified_at = current_timestamp;

        require!(
            !server_vault.server_id.is_empty(),
            CustomError::InvalidServerId
        );
        require!(
            !server_vault.server_name.is_empty(),
            CustomError::InvalidServerName
        );

        Ok(())
    }

    pub fn update_server_vault_authority<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateServerVaultAuthority<'info>>,
    ) -> Result<()> {
        require!(
            ctx.accounts.current_authority.key()
                == ctx.light_accounts.server_vault.current_authority,
            CustomError::InvalidAuthority
        );

        let clock = Clock::get()?;
        let server_vault = &mut ctx.light_accounts.server_vault;
        server_vault.modified_at = clock.unix_timestamp;

        Ok(())
    }

    pub fn initialize_escrow_vault<'info>(
        ctx: LightContext<'_, '_, '_, 'info, InitializeEscrowVault<'info>>,
    ) -> Result<()> {
        Ok(())
    }
    pub fn update_escrow_vault<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateEscrowVaultAuthority<'info>>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn initialize_airdrop_vault<'info>(
        ctx: LightContext<'_, '_, '_, 'info, InitializeAirdropVault<'info>>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn update_airdrop_vault<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateAirdropVaultAuthority<'info>>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn create_compressed_mint<'info>(
        ctx: Context<CreateCompressedMint>,
        name: String,
        symbol: String,
        decimals: u8,
        uri: String,
        nonce: u16,
    ) -> Result<()> {
        user::create_compressed_mint::create_compressed_mint(
            ctx, name, symbol, decimals, uri, nonce,
        )
    }

    pub fn compress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, CompressTokens<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        user::compress_tokens(ctx, inputs)
    }

    pub fn mint_tokens<'info>(
        ctx: Context<MintTokens>,
        public_keys: Vec<Pubkey>,
        amounts: Vec<u64>,
        lamports: Option<u64>,
    ) -> Result<()> {
        user::mint_tokens(ctx, public_keys, amounts, lamports)
    }

    pub fn transfer_compressed_tokens<'info>(
        ctx: Context<'_, '_, '_, '_, TransferCompressedTokensWallet<'_>>,
        transfer_inputs: Vec<u8>,
    ) -> Result<()> {
        transfer_compressed_tokens::transfer_compressed_tokens_wallet(ctx, transfer_inputs)
    }

    pub fn transfer_compressed_tokens_with_pda<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferCompressedTokensWithPda<'info>>,
        amount: u64,
        proof: CompressedProof,
        mint: Pubkey,
        input_token_data_with_context: Vec<InputTokenDataWithContext>,
        output_state_merkle_tree_account_indices: Vec<u8>,
        cpi_context: CompressedCpiContext,
    ) -> Result<()> {
        user_vault_transfer::transfer_compressed_tokens(
            ctx,
            amount,
            proof,
            mint,
            input_token_data_with_context,
            output_state_merkle_tree_account_indices,
            cpi_context,
        )
    }

    pub fn decompress_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, DecompressTokens<'info>>,
        inputs: Vec<u8>,
    ) -> Result<()> {
        user::decompress_tokens(ctx, inputs)
    }

    /// * ctx: CreateUser ctx
    ///
    pub fn create_user<'info>(
        ctx: LightContext<'_, '_, '_, 'info, CreateUser<'info>>,
        input: UserAccount,
        username: String,
    ) -> Result<()> {
        ctx.light_accounts.user_vault.authority = ctx.accounts.signer.key();
        *ctx.light_accounts.user_vault = input;
        ctx.light_accounts.user_vault.username = username;
        Ok(())
    }

    pub fn update_user_profile<'info>(
        ctx: LightContext<'_, '_, '_, 'info, UpdateUserProfile<'info>>,
        new_profile_effect: Option<Pubkey>,
        new_theme: Option<Pubkey>,
    ) -> Result<()> {
        if let Some(effect) = new_profile_effect {
            ctx.light_accounts.user_account.profile_effect = Some(effect);
        }
        if let Some(theme) = new_theme {
            ctx.light_accounts.user_account.theme = Some(theme);
        }
        Ok(())
    }

    pub fn create_user_handle<'info>(
        ctx: LightContext<'_, '_, '_, 'info, CreateUserHandle<'info>>,
        handle: String,
    ) -> Result<()> {
        ctx.light_accounts.handle_asset.owner = ctx.accounts.signer.key();
        ctx.light_accounts.handle_asset.asset_type = AssetType::UserHandle;
        ctx.light_accounts.handle_asset.data = handle;
        ctx.light_accounts.user_account.handle = Some(ctx.light_accounts.handle_asset.owner.key());
        Ok(())
    }

    pub fn transfer_user_handle<'info>(
        ctx: LightContext<'_, '_, '_, 'info, TransferUserHandle<'info>>,
    ) -> Result<()> {
        ctx.light_accounts.handle_asset.owner = ctx.light_accounts.to_user_account.authority.key();
        ctx.light_accounts.from_user_account.handle = None;
        ctx.light_accounts.to_user_account.handle =
            Some(ctx.light_accounts.handle_asset.owner.key());
        Ok(())
    }

    // Server Creation and Ownership
    pub fn create_server<'info>(
        ctx: LightContext<'_, '_, '_, 'info, CreateServer<'info>>,
        name: String,
        ticker: String,
    ) -> Result<()> {
        // Generate and associate a unique on-chain MPC-based signer with the server
        // This is handled by the Light Protocol, we just need to set the authority
        ctx.light_accounts.server_account.authority = ctx.accounts.signer.key();
        ctx.light_accounts.server_account.name = name;
        ctx.light_accounts.server_account.ticker = ticker;

        // Mint an NFT representing server ownership
        // TODO

        Ok(())
    }

    // Server Ownership Transfer
    pub fn transfer_server_ownership<'info>(
        ctx: LightContext<'_, '_, '_, 'info, TransferServerOwnership<'info>>,
        new_owner: Pubkey,
    ) -> Result<()> {
        ctx.light_accounts.server_account.authority = new_owner;

        Ok(())
    }
}
