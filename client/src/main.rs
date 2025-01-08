use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use solana_client::nonblocking::rpc_client::RpcClient;
use zk_client::{
    builder::admin::vault_config::{
        initialize_and_manage_vault_test, initialize_vault_config_test,
    },
    user::{
        compress_tokens::create_compress_tokens_inx,
        compressed_mint,
        cpda_tokens_transfer::create_user_vaults_transfer,
        server_vault::{initialize_server_vault, update_server_vault},
        user_vault::{initialize_user_vault, update_user_vault},
    },
};

#[derive(Debug, Parser)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: SodaCommands,
}

#[derive(Debug, Parser)]
pub enum SodaCommands {
    #[clap(subcommand)]
    VaultConfigScenarios(VaultConfigScenarios),
    InitializeUserVault,
    InitializeServerVault,
    UpdateUserVault,
    UpdateServerVault,
    CreateCompressedMint,
    CreateCompressTokens,
    TransferCompressedTokens,
}

#[derive(Debug, Parser)]
pub enum InitializeUserVault {
    InitUserVault {},
}
#[derive(Debug, Parser)]
pub enum VaultConfigScenarios {
    /// Run a complete initialization and authority update test sequence
    InitAndAuthority {
        /// Number of initial service signers (1-3)
        #[clap(value_parser = clap::value_parser!(u8).range(1..=3))]
        num_signers: Option<u8>,
    },
    /// Run a complete initialization and service signer management sequence
    InitAndManageSigners {
        /// Number of initial service signers (1-3)
        #[clap(value_parser = clap::value_parser!(u8).range(1..=3))]
        num_signers: Option<u8>,
    },
}

#[derive(Debug, Parser)]
pub enum InitializeVaultConfigCommands {
    /// Regular initialization with default settings
    Initialize,
}

#[derive(Debug, Parser)]
pub enum UpdateVaultAuthorityCommands {
    /// Regular update of vault authority
    Update,
}

#[derive(Debug, Parser)]
pub enum ManageServiceSignerCommands {
    /// Add a new service signer
    Add,
    /// Remove an existing service signer
    Remove,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let rpc_client = RpcClient::new(
        "https://devnet.helius-rpc.com/?api-key=16ef3f61-7567-47d9-9c44-edec13422455".to_string(),
        //"http://127.0.0.1:8899".to_string(),
    );

    let opts = Opts::parse();
    match opts.command {
        SodaCommands::VaultConfigScenarios(scenario) => match scenario {
            VaultConfigScenarios::InitAndAuthority { num_signers } => {
                initialize_vault_config_test(num_signers, rpc_client).await?;
            }

            VaultConfigScenarios::InitAndManageSigners { num_signers } => {
                initialize_and_manage_vault_test(num_signers, rpc_client).await?;
            }
        },
        SodaCommands::InitializeUserVault {} => {
            initialize_user_vault(&rpc_client, None, None).await?;
        }
        SodaCommands::InitializeServerVault {} => {
            initialize_server_vault(&rpc_client, None, None).await?;
        }
        SodaCommands::UpdateUserVault {} => {
            update_user_vault(rpc_client).await?;
        }
        SodaCommands::UpdateServerVault {} => {
            update_server_vault(rpc_client).await?;
        }
        SodaCommands::CreateCompressedMint {} => {
            compressed_mint::create_compressed_mint(rpc_client).await?;
        }
        SodaCommands::CreateCompressTokens {} => {
            create_compress_tokens_inx(rpc_client).await?;
        }
        SodaCommands::TransferCompressedTokens {} => {
            create_user_vaults_transfer(Arc::new(rpc_client)).await?;
        }
    }

    Ok(())
}
