use std::rc::Rc;

use crate::settings::config::load_cfg;
use crate::settings::config::ClientConfig;
use anchor_client::{Client, Cluster};
use anchor_lang::prelude::AccountMeta;
use anyhow::Result;

use zk_onchain::state::*;
use zk_onchain::vaults::config_authority;
use zk_onchain::vaults::ServiceSignerOperation;
use zk_onchain::{accounts as soda_accounts, instruction as soda_instructions};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

/// Initialize a new vault config with service signers
pub fn initialize_vault_config(
    config: &ClientConfig,
    service_signers: Vec<Pubkey>,
) -> Result<Vec<Instruction>> {
    let payer = read_keypair_file(&config.payer_path).unwrap();
    let url = Cluster::Custom(config.http_url.clone(), config.ws_url.clone());

    // Client
    let client = Client::new(url, Rc::new(payer));
    let program = client.program(zk_onchain::id())?;

    // Find config PDA
    let (config_pda, _) = Pubkey::find_program_address(
        &[
            VAULT_CONFIG_SEED.as_bytes(),
            program.payer().to_bytes().as_ref(),
        ],
        &zk_onchain::id(),
    );

    let remaining_accounts: Vec<AccountMeta> = service_signers
        .iter()
        .map(|pubkey| AccountMeta {
            pubkey: *pubkey,
            is_signer: true,
            is_writable: false,
        })
        .collect();

    let compute = ComputeBudgetInstruction::set_compute_unit_limit(100000);
    let price = ComputeBudgetInstruction::set_compute_unit_price(100000);

    let instructions = program
        .request()
        .accounts(soda_accounts::InitializeConfig {
            config: config_pda,
            payer: program.payer(),
            authority: config_authority::id(),
            system_program: system_program::id(),
        })
        .args(soda_instructions::InitializeVaultConfig {})
        .instruction(compute)
        .instruction(price)
        .accounts(remaining_accounts)
        //  .send();
        .instructions()?;

    Ok(instructions)
}

/// Update the vault authority
pub fn update_vault_authority(
    config: &ClientConfig,
    current_authority: Pubkey,
    new_authority: Pubkey,
) -> Result<Vec<Instruction>> {
    let payer = read_keypair_file(&config.payer_path).unwrap();
    let url = Cluster::Custom(config.http_url.clone(), config.ws_url.clone());

    // Client
    let client = Client::new(url, Rc::new(payer));
    let program = client.program(zk_onchain::id())?;

    // Find config PDA
    let (config_pda, _) = Pubkey::find_program_address(
        &[
            VAULT_CONFIG_SEED.as_bytes(),
            program.payer().to_bytes().as_ref(),
        ],
        &zk_onchain::id(),
    );

    let instructions = program
        .request()
        .accounts(soda_accounts::UpdateVaultAuthority {
            config: config_pda,
            current_authority,
            new_authority,
        })
        .args(soda_instructions::UpdateVaultAuthority {})
        .instructions()?;

    Ok(instructions)
}

/// Manage (add/remove) service signers
pub fn manage_service_signer(
    config: &ClientConfig,
    authority: Pubkey,
    service_signer: Pubkey,
    operation: ServiceSignerOperation,
) -> Result<Vec<Instruction>> {
    let payer = read_keypair_file(&config.payer_path).unwrap();
    let url = Cluster::Custom(config.http_url.clone(), config.ws_url.clone());

    // Client
    let client = Client::new(url, Rc::new(payer));
    let program = client.program(zk_onchain::id())?;

    // Find config PDA
    let (config_pda, _) = Pubkey::find_program_address(
        &[
            VAULT_CONFIG_SEED.as_bytes(),
            program.payer().to_bytes().as_ref(),
        ],
        &zk_onchain::id(),
    );

    let instructions = program
        .request()
        .accounts(soda_accounts::ManageServiceSigner {
            config: config_pda,
            authority,
            service_signer,
        })
        .args(soda_instructions::ManageServiceSigner { operation })
        .instructions()?;

    Ok(instructions)
}

pub async fn initialize_vault_config_test(
    num_signers: Option<u8>,
    rpc_client: RpcClient,
) -> anyhow::Result<()> {
    // Try to get keypair path from environment variable first
    let payer_path = std::env::var("SOLANA_KEYPAIR_PATH").unwrap_or_else(|_| {
        // If not set, use default path
        let default_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
        default_path
    });
    let client_config = "client_config.ini";
    let config = load_cfg(&client_config.to_string()).unwrap();
    let payer = read_keypair_file(&payer_path).unwrap();

    let initial_signers = num_signers.unwrap_or(1);
    log::info!(
        "Running Full Test Scenario with {} signers",
        initial_signers
    );

    let mut instructions: Vec<solana_sdk::instruction::Instruction> = Vec::new();

    // Create and store all keypairs that will be needed
    let init_service_signers: Vec<Keypair> = (0..initial_signers).map(|_| Keypair::new()).collect();
    let additional_signers: Vec<Keypair> = (0..2).map(|_| Keypair::new()).collect();
    let new_authority = Keypair::new();

    // 1. Initialize with signers
    instructions.extend(initialize_vault_config(
        &config,
        init_service_signers.iter().map(|kp| kp.pubkey()).collect(),
    )?);

    // 2. Update authority
    instructions.extend(update_vault_authority(
        &config,
        payer.pubkey(),
        new_authority.pubkey(),
    )?);

    // 3. Add more signers
    for service_signer in &additional_signers {
        instructions.extend(manage_service_signer(
            &config,
            new_authority.pubkey(),
            service_signer.pubkey(),
            ServiceSignerOperation::Add,
        )?);
    }

    // 4. Remove a signer
    if !additional_signers.is_empty() {
        instructions.extend(manage_service_signer(
            &config,
            new_authority.pubkey(),
            additional_signers[0].pubkey(),
            ServiceSignerOperation::Remove,
        )?);
    }

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    // Collect all signers needed for the transaction
    let mut all_signers = vec![&payer, &new_authority];
    all_signers.extend(init_service_signers.iter());
    all_signers.extend(additional_signers.iter());

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::processed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..Default::default()
            },
        )
        .await?;

    println!("Transaction signature: {}", signature);

    Ok(())
}

pub async fn initialize_and_manage_vault_test(
    num_signers: Option<u8>,
    rpc_client: RpcClient,
) -> anyhow::Result<()> {
    // Try to get keypair path from environment variable first
    let payer_path = std::env::var("SOLANA_KEYPAIR_PATH").unwrap_or_else(|_| {
        // If not set, use default path
        let default_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
        default_path
    });
    let client_config = "client_config.ini";
    let config = load_cfg(&client_config.to_string()).unwrap();
    let payer = read_keypair_file(&payer_path).unwrap();

    let initial_signers = num_signers.unwrap_or(1);

    let mut instructions: Vec<solana_sdk::instruction::Instruction> = Vec::new();

    // Create all keypairs upfront
    let init_service_signers: Vec<Keypair> = (0..initial_signers).map(|_| Keypair::new()).collect();
    let additional_signers: Vec<Keypair> = (0..2).map(|_| Keypair::new()).collect();
    let new_authority = Keypair::new();

    // 1. Initialize with signers
    instructions.extend(initialize_vault_config(
        &config,
        init_service_signers.iter().map(|kp| kp.pubkey()).collect(),
    )?);

    // 2. Update authority
    instructions.extend(update_vault_authority(
        &config,
        payer.pubkey(),
        new_authority.pubkey(),
    )?);

    // 3. Add more signers
    for service_signer in &additional_signers {
        instructions.extend(manage_service_signer(
            &config,
            new_authority.pubkey(),
            service_signer.pubkey(),
            ServiceSignerOperation::Add,
        )?);
    }

    // 4. Remove a signer
    if !additional_signers.is_empty() {
        instructions.extend(manage_service_signer(
            &config,
            new_authority.pubkey(),
            additional_signers[0].pubkey(),
            ServiceSignerOperation::Remove,
        )?);
    }

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    // Collect all signers in the correct order
    let mut all_signers = vec![&payer, &new_authority];
    all_signers.extend(init_service_signers.iter());
    all_signers.extend(additional_signers.iter());

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &all_signers,
        recent_blockhash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            CommitmentConfig::processed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                ..Default::default()
            },
        )
        .await?;

    println!("Transaction signature: {}", signature);

    Ok(())
}
