use crate::state::*;
use anchor_lang::prelude::*;

pub const MAX_SERVICE_SIGNERS: usize = 3;
pub mod config_authority {
    use anchor_lang::prelude::declare_id;
    #[cfg(feature = "devnet")]
    declare_id!("Bn6jUQPC48meSkE5nZ8G8yWyxsuoiGwQwyX127nVmWWZ");
    #[cfg(not(feature = "devnet"))]
    declare_id!("Bn6jUQPC48meSkE5nZ8G8yWyxsuoiGwQwyX127nVmWWZ");
}

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(
        init,
        payer = payer,
        space = VaultConfigState::LEN,
        seeds = [VAULT_CONFIG_SEED.as_bytes(), payer.key().as_ref()],
        bump
    )]
    pub config: AccountLoader<'info, VaultConfigState>,

    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        constraint = (authority.key() == config_authority::id()) @ CustomError::InvalidSigner
    )]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[account(zero_copy(unsafe))]
#[repr(packed)]
#[derive(Default)]
pub struct VaultConfigState {
    pub service_signers: [Pubkey; MAX_SERVICE_SIGNERS],
    pub service_signers_count: u8,
    pub created_at: i64,
    pub created_by: Pubkey,
    pub modified_at: i64,
    pub modified_by: Pubkey,
    pub current_update_authority: Pubkey,
}

pub fn initialize_vault_config(ctx: Context<InitializeConfig>) -> Result<()> {
    let config = &mut ctx.accounts.config.load_init()?;
    let clock = Clock::get()?;

    // Initialize empty service_signers array
    config.service_signers = [Pubkey::default(); MAX_SERVICE_SIGNERS];
    config.service_signers_count = 0;

    // Set initial metadata
    config.created_at = clock.unix_timestamp;
    config.created_by = ctx.accounts.payer.key();
    config.modified_at = clock.unix_timestamp;
    config.modified_by = ctx.accounts.payer.key();
    config.current_update_authority = ctx.accounts.authority.key();

    Ok(())
}

impl VaultConfigState {
    pub const LEN: usize = 8 +  // discriminator
        (32 * MAX_SERVICE_SIGNERS) +  // service_signers array
        1 +          // service_signers_count
        8 +          // created_at
        32 +         // created_by
        8 +          // modified_at
        32 +         // modified_by
        32; // current_update_authority
}

#[derive(Accounts)]
pub struct ManageServiceSigner<'info> {
    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED.as_bytes(), config.load()?.created_by.as_ref()],
        bump
    )]
    pub config: AccountLoader<'info, VaultConfigState>,

    #[account(
        constraint = (authority.key() == config.load()?.current_update_authority) @ CustomError::InvalidAuthority
    )]
    pub authority: Signer<'info>,

    /// The service signer being added or removed
    pub service_signer: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub enum ServiceSignerOperation {
    Add,
    Remove,
}

pub fn manage_service_signer(
    ctx: Context<ManageServiceSigner>,
    operation: ServiceSignerOperation,
) -> Result<()> {
    let config = &mut ctx.accounts.config.load_mut()?;
    let clock = Clock::get()?;
    let service_signer_key = ctx.accounts.service_signer.key();

    match operation {
        ServiceSignerOperation::Add => {
            require!(
                config.service_signers_count < MAX_SERVICE_SIGNERS as u8,
                CustomError::TooManyServiceSigners
            );

            require!(
                !config.is_service_signer(&service_signer_key),
                CustomError::DuplicateServiceSigner
            );

            let current_count = config.service_signers_count as usize;
            config.service_signers[current_count] = service_signer_key;
            config.service_signers_count += 1;
        }
        ServiceSignerOperation::Remove => {
            let current_count = config.service_signers_count as usize;
            let position = config.service_signers[..current_count]
                .iter()
                .position(|signer| signer == &service_signer_key)
                .ok_or(CustomError::InvalidSignerExist)?;

            // Shift remaining signers left to fill the gap
            for i in position..(current_count - 1) {
                config.service_signers[i] = config.service_signers[i + 1];
            }

            // Clear the last position and decrement count
            config.service_signers[current_count - 1] = Pubkey::default();
            config.service_signers_count -= 1;
        }
    }

    // Update modification metadata
    config.modified_at = clock.unix_timestamp;
    config.modified_by = ctx.accounts.authority.key();

    Ok(())
}

impl VaultConfigState {
    pub fn is_service_signer(&self, pubkey: &Pubkey) -> bool {
        self.service_signers[..self.service_signers_count as usize]
            .iter()
            .any(|signer| signer == pubkey)
    }
}

#[derive(Accounts)]
pub struct UpdateVaultAuthority<'info> {
    #[account(
        mut,
        seeds = [VAULT_CONFIG_SEED.as_bytes(), config.load()?.created_by.as_ref()],
        bump
    )]
    pub config: AccountLoader<'info, VaultConfigState>,

    #[account(
        constraint = (current_authority.key() == config.load()?.current_update_authority) @ CustomError::InvalidAuthority
    )]
    pub current_authority: Signer<'info>,

    pub new_authority: Signer<'info>,
}

pub fn update_vault_authority(ctx: Context<UpdateVaultAuthority>) -> Result<()> {
    let config = &mut ctx.accounts.config.load_mut()?;
    let clock = Clock::get()?;

    // Update the authority
    config.current_update_authority = ctx.accounts.new_authority.key();

    // Update modification metadata
    config.modified_at = clock.unix_timestamp;
    config.modified_by = ctx.accounts.current_authority.key();

    Ok(())
}
