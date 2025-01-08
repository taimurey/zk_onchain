use anchor_lang::AnchorDeserialize;

use light_sdk::address::derive_address;
use light_sdk::address::derive_address_seed;
use light_sdk::merkle_context::pack_address_merkle_context;
use light_sdk::merkle_context::pack_merkle_context;
use light_sdk::merkle_context::AddressMerkleContext;
use light_sdk::merkle_context::MerkleContext;
use light_sdk::merkle_context::PackedAddressMerkleContext;
use light_sdk::merkle_context::PackedMerkleContext;
use light_sdk::merkle_context::RemainingAccounts;
use light_sdk::proof::CompressedProof;
use light_sdk::utils::get_cpi_authority_pda;
use light_sdk::verify::find_cpi_signer;
use light_sdk::PROGRAM_ID_ACCOUNT_COMPRESSION;
use light_sdk::PROGRAM_ID_LIGHT_SYSTEM;

use photon_api::apis::configuration::{ApiKey, Configuration};
use photon_api::apis::default_api::get_compressed_accounts_by_owner_post;
use photon_api::apis::default_api::get_validity_proof_post;
use photon_api::models::GetCompressedAccountsByOwnerPostRequest;
use photon_api::models::GetCompressedAccountsByOwnerPostRequestParams;
use photon_api::models::{
    GetValidityProofPost200Response, GetValidityProofPostRequest, GetValidityProofPostRequestParams,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::bs58;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Keypair;
use solana_sdk::transaction::Transaction;
use std::str::FromStr;
use std::sync::Arc;
use zk_onchain::vaults::UserVaultState;

use crate::settings::config::load_cfg;
use crate::settings::config::ClientConfig;
use crate::utils::config::keypair_1;
use crate::utils::config::keypair_2;
use crate::utils::config::keypair_3;
use crate::utils::vectorizer::vec_to_array;
use anchor_client::{Client, Cluster};
use anyhow::Result;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signer::Signer;
use solana_sdk::system_program;
use zk_onchain::state::*;
use zk_onchain::vaults::config_authority;
use zk_onchain::{accounts as soda_accounts, instruction as soda_instructions};

pub fn derive_user_vault(
    authority: Pubkey,
    address_merkle_context: AddressMerkleContext,
) -> [u8; 32] {
    let address_seed = derive_address_seed(
        &[USER_VAULT.as_bytes(), authority.as_ref()],
        &zk_onchain::ID,
    );

    derive_address(&address_seed, &address_merkle_context)
}

/// Implementation for Solana RpcConnection Error
// Get Merkle account parameters and proof
async fn get_account_params(
    current_authority: Pubkey,
    user_vault: Option<Pubkey>,
) -> Result<(
    u16,
    PackedMerkleContext,
    PackedAddressMerkleContext,
    CompressedProof,
    RemainingAccounts,
)> {
    // Initialize account pubkeys
    let merkle_tree_pubkey = Pubkey::from_str("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT")?;
    let address_merkle_tree_pubkey =
        Pubkey::from_str("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2")?;
    let nullifier_queue_pubkey = Pubkey::from_str("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148")?;
    let address_merkle_tree_queue_pubkey =
        Pubkey::from_str("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F")?;

    let mut remaining_accounts = RemainingAccounts::default();

    // Setup contexts
    let merkle_context = pack_merkle_context(
        MerkleContext {
            merkle_tree_pubkey,
            nullifier_queue_pubkey,
            leaf_index: 0,
            queue_index: None,
        },
        &mut remaining_accounts,
    );

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey: address_merkle_tree_queue_pubkey,
    };

    let user_vault = user_vault.unwrap_or(Pubkey::from(derive_user_vault(
        current_authority,
        address_merkle_context,
    )));
    let address_string = bs58::encode(user_vault).into_string();

    let address_merkle_context =
        pack_address_merkle_context(address_merkle_context, &mut remaining_accounts);

    // Get proof
    let rpc_config = Configuration {
        base_path: "https://devnet.helius-rpc.com/".into(),
        api_key: Some(ApiKey {
            prefix: None,
            key: "16ef3f61-7567-47d9-9c44-edec13422455".into(),
        }),
        ..Configuration::default()
    };

    let proof_result = get_validity_proof_post(
        &rpc_config,
        GetValidityProofPostRequest {
            params: Box::new(GetValidityProofPostRequestParams {
                new_addresses: Some(vec![address_string]),
                new_addresses_with_trees: None,
                hashes: None,
            }),
            ..Default::default()
        },
    )
    .await?;

    let (compressed_proof, root_indices) = get_proof(proof_result.clone()).await?;

    Ok((
        root_indices,
        merkle_context,
        address_merkle_context,
        compressed_proof,
        remaining_accounts,
    ))
}

pub async fn get_proof(
    rpc_result: GetValidityProofPost200Response,
) -> Result<(CompressedProof, u16)> {
    let mut compressed_proof: CompressedProof = CompressedProof {
        a: [0; 32],
        b: [0; 64],
        c: [0; 32],
    };
    let mut root_indices: u16 = 0;

    if let Some(result_box) = rpc_result.result {
        let result = *result_box.value.compressed_proof;

        // Convert a, b, and c from Vec<u8> to [u8; 64]
        let a_array: [u8; 32] = vec_to_array(result.a, "a").unwrap();
        let b_array: [u8; 64] = vec_to_array(result.b, "b").unwrap();
        let c_array: [u8; 32] = vec_to_array(result.c, "c").unwrap();

        root_indices = result_box.value.root_indices[0] as u16;

        // Create the CompressedProof struct with the converted values
        compressed_proof = CompressedProof {
            a: a_array, //  a (32 bytes)
            b: b_array, //  b (64 bytes)
            c: c_array, //  c (32 bytes)
        };
    } else {
        // Handle the case where `result` is None
        eprintln!("Error: rpc_result.result is None");

        compressed_proof = CompressedProof {
            a: [0; 32], // Default value
            b: [0; 64], // Default value
            c: [0; 32], // Default value
        };
    }

    Ok((compressed_proof, root_indices))
}

// Get PDAs and program addresses
pub fn get_program_addresses() -> Result<(Pubkey, Pubkey, Pubkey)> {
    let config_pda = Pubkey::find_program_address(
        &[
            VAULT_CONFIG_SEED.as_bytes(),
            config_authority::id().as_ref(),
        ],
        &zk_onchain::id(),
    )
    .0;

    let registered_program_pda = Pubkey::find_program_address(
        &[PROGRAM_ID_LIGHT_SYSTEM.to_bytes().as_slice()],
        &PROGRAM_ID_ACCOUNT_COMPRESSION,
    )
    .0;

    let account_compression_authority = get_cpi_authority_pda(&PROGRAM_ID_LIGHT_SYSTEM);

    Ok((
        config_pda,
        registered_program_pda,
        account_compression_authority,
    ))
}

// Build vault instructions
async fn build_instructions(
    config: &ClientConfig,
    payer: &Arc<Keypair>,
    current_authority: &Keypair,
    service_signer: &Keypair,
    params: (
        u16,
        PackedMerkleContext,
        PackedAddressMerkleContext,
        CompressedProof,
        RemainingAccounts,
    ),
) -> Result<Vec<Instruction>> {
    // Setup client
    let client = Client::new_with_options(
        Cluster::Custom(config.http_url.clone(), config.ws_url.clone()),
        payer.clone(),
        CommitmentConfig::processed(),
    );
    let program = client.program(zk_onchain::id())?;

    let (config_pda, registered_program_pda, account_compression_authority) =
        get_program_addresses()?;

    let cpi_signer = find_cpi_signer(&zk_onchain::ID);

    let (
        root_indices,
        merkle_context,
        address_merkle_context,
        compressed_proof,
        remaining_accounts,
    ) = params;

    // Config init instruction
    let config_ix = program
        .request()
        .accounts(soda_accounts::InitializeConfig {
            config: config_pda,
            payer: payer.pubkey(),
            authority: config_authority::id(),
            system_program: system_program::id(),
        })
        .args(soda_instructions::InitializeVaultConfig)
        .instructions()?;

    // Vault init instruction
    let mut init_vault_ix = program
        .request()
        .accounts(soda_accounts::InitializeUserVault {
            payer: payer.pubkey(),
            self_program: zk_onchain::id(),
            service_signer: service_signer.pubkey(),
            current_authority: current_authority.pubkey(),
            cpi_signer,
            config: config_pda,
            config_authority: config_authority::id(),
            /* Light Accounts */
            system_program: system_program::id(),
            light_system_program: PROGRAM_ID_LIGHT_SYSTEM,
            account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
            registered_program_pda,
            noop_program: light_sdk::PROGRAM_ID_NOOP,
            account_compression_authority,
        })
        .args(soda_instructions::InitializeUserVault {
            proof: compressed_proof,
            inputs: Vec::new(),
            merkle_context,
            address_merkle_context,
            address_merkle_tree_root_index: root_indices,
            merkle_tree_root_index: 0,
        })
        .instructions()?;

    init_vault_ix[0]
        .accounts
        .extend(remaining_accounts.to_account_metas());

    // Combine with compute budget instructions
    let mut instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1000000000),
        ComputeBudgetInstruction::set_compute_unit_price(100000),
    ];
    // instructions.extend(config_ix);
    instructions.extend(init_vault_ix);

    Ok(instructions)
}

// Main vault initialization function
pub async fn initialize_user_vault(
    rpc_client: &RpcClient,
    authority: Option<Arc<Keypair>>,
    user_vault: Option<Pubkey>,
) -> Result<()> {
    let client_config = "client_config.ini";
    let config = &load_cfg(&client_config.to_string()).unwrap();
    let current_authority = authority.unwrap_or(Arc::new(keypair_1()));
    // Create new keypairs for each role
    let payer = Arc::new(read_keypair_file(&config.payer_path).unwrap());
    let service_signer = keypair_2();

    // Get account parameters and proof
    let params = get_account_params(current_authority.pubkey(), user_vault).await?;

    // Build instructions
    let instructions =
        build_instructions(config, &payer, &current_authority, &service_signer, params).await?;

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[&payer, &current_authority, &service_signer],
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

async fn get_update_light_account_params(
    current_authority: Pubkey,
) -> Result<(
    u16,
    PackedMerkleContext,
    PackedAddressMerkleContext,
    CompressedProof,
    RemainingAccounts,
    Vec<u8>,
)> {
    // Initialize account pubkeys
    let merkle_tree_pubkey = Pubkey::from_str("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT")?;
    let address_merkle_tree_pubkey =
        Pubkey::from_str("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2")?;
    let nullifier_queue_pubkey = Pubkey::from_str("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148")?;
    let address_merkle_tree_queue_pubkey =
        Pubkey::from_str("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F")?;

    let mut remaining_accounts = RemainingAccounts::default();

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey: address_merkle_tree_queue_pubkey,
    };

    // Generate address
    let address_seed = derive_address_seed(
        &[USER_VAULT.as_bytes(), current_authority.as_ref()],
        &zk_onchain::ID,
    );

    let address = derive_address(&address_seed, &address_merkle_context);
    let address_string = bs58::encode(address).into_string();

    let address_merkle_context =
        pack_address_merkle_context(address_merkle_context, &mut remaining_accounts);

    // Get proof
    let rpc_config = Configuration {
        base_path: "https://devnet.helius-rpc.com/".into(),
        api_key: Some(ApiKey {
            prefix: None,
            key: "16ef3f61-7567-47d9-9c44-edec13422455".into(),
        }),
        ..Configuration::default()
    };

    let compressed_accounts = get_compressed_accounts_by_owner_post(
        &rpc_config,
        GetCompressedAccountsByOwnerPostRequest {
            params: Box::new(GetCompressedAccountsByOwnerPostRequestParams {
                owner: zk_onchain::ID.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;

    let mut decoded_bytes = Vec::new();

    let mut item_account = photon_api::models::Account::default();
    if let Some(compressed_account) = compressed_accounts.result {
        item_account = compressed_account.value.items[0].clone();
        let compressed_data = item_account.data.clone().unwrap().data;

        // First decode the base58 string into a Vec<u8>
        decoded_bytes = base64::decode(compressed_data.clone())?;
        // Create a mutable slice reference
        let user_vault_state = UserVaultState::try_from_slice(&decoded_bytes)?;

        println!("User Vault: {user_vault_state:#?}");
    }

    let proof_result = get_validity_proof_post(
        &rpc_config,
        GetValidityProofPostRequest {
            params: Box::new(GetValidityProofPostRequestParams {
                new_addresses: None,
                new_addresses_with_trees: None,
                hashes: Some(vec![item_account.hash.clone()]),
            }),
            ..Default::default()
        },
    )
    .await?;

    let mut compressed_proof: CompressedProof = CompressedProof {
        a: [0; 32],
        b: [0; 64],
        c: [0; 32],
    };

    let mut root_indices: u16 = 0;
    let mut merkle_context = PackedMerkleContext::default();
    if let Some(result_box) = proof_result.result {
        let result = *result_box.value.compressed_proof;

        // Convert a, b, and c from Vec<u8> to [u8; 64]
        let a_array: [u8; 32] = vec_to_array(result.a, "a").unwrap();
        let b_array: [u8; 64] = vec_to_array(result.b, "b").unwrap();
        let c_array: [u8; 32] = vec_to_array(result.c, "c").unwrap();

        root_indices = result_box.value.root_indices[0] as u16;

        // Setup contexts
        merkle_context = pack_merkle_context(
            MerkleContext {
                merkle_tree_pubkey,
                nullifier_queue_pubkey,
                leaf_index: result_box.value.leaf_indices[0] as u32,
                queue_index: None,
            },
            &mut remaining_accounts,
        );

        // Create the CompressedProof struct with the converted values
        compressed_proof = CompressedProof {
            a: a_array, //  a (32 bytes)
            b: b_array, //  b (64 bytes)
            c: c_array, //  c (32 bytes)
        };
    } else {
        // Handle the case where `result` is None
        eprintln!("Error: rpc_result.result is None");

        compressed_proof = CompressedProof {
            a: [0; 32], // Default value
            b: [0; 64], // Default value
            c: [0; 32], // Default value
        };
    }

    Ok((
        root_indices,
        merkle_context,
        address_merkle_context,
        compressed_proof,
        remaining_accounts,
        decoded_bytes,
    ))
}

async fn update_user_vault_instructions(
    config: &ClientConfig,
    payer: &Arc<Keypair>,
    current_authority: &Keypair,
    new_authority: &Keypair,
    service_signer: &Keypair,
    params: (
        u16,
        PackedMerkleContext,
        PackedAddressMerkleContext,
        CompressedProof,
        RemainingAccounts,
        Vec<u8>,
    ),
) -> Result<Vec<Instruction>> {
    // Setup client
    let client = Client::new_with_options(
        Cluster::Custom(config.http_url.clone(), config.ws_url.clone()),
        payer.clone(),
        CommitmentConfig::processed(),
    );
    let program = client.program(zk_onchain::id())?;

    let (config_pda, registered_program_pda, account_compression_authority) =
        get_program_addresses()?;

    let cpi_signer = find_cpi_signer(&zk_onchain::ID);

    let (
        root_indices,
        merkle_context,
        address_merkle_context,
        compressed_proof,
        remaining_accounts,
        compressed_inputs,
    ) = params;

    // upate-user-vault-instruction
    let mut update_user_vault_ix = program
        .request()
        .accounts(soda_accounts::UpdateUserVaultAuthority {
            payer: payer.pubkey(),
            self_program: zk_onchain::id(),
            service_signer: service_signer.pubkey(),
            current_authority: current_authority.pubkey(),
            new_authority: new_authority.pubkey(),
            cpi_signer,
            /* Light Accounts */
            system_program: system_program::id(),
            light_system_program: PROGRAM_ID_LIGHT_SYSTEM,
            account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
            registered_program_pda,
            noop_program: light_sdk::PROGRAM_ID_NOOP,
            account_compression_authority,
        })
        .args(soda_instructions::UpdateUserVaultAuthority {
            proof: compressed_proof,
            inputs: vec![compressed_inputs],
            merkle_context,
            address_merkle_context,
            address_merkle_tree_root_index: 0,
            merkle_tree_root_index: root_indices,
        })
        .instructions()?;

    update_user_vault_ix[0]
        .accounts
        .extend(remaining_accounts.to_account_metas());

    // Combine with compute budget instructions
    let mut instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1000000000),
        ComputeBudgetInstruction::set_compute_unit_price(100000),
    ];

    instructions.extend(update_user_vault_ix);

    Ok(instructions)
}

// Main vault initialization function
pub async fn update_user_vault(rpc_client: RpcClient) -> Result<()> {
    let client_config = "client_config.ini";
    let config = load_cfg(&client_config.to_string()).unwrap();
    // Submit transaction

    let current_authority = keypair_1();
    let service_signer = keypair_2();
    let new_authority = keypair_3();
    // Create new keypairs for each role

    let payer = Arc::new(read_keypair_file(&config.payer_path).unwrap());

    // Get account parameters and proof
    let params = get_update_light_account_params(current_authority.pubkey()).await?;

    // Build instructions
    let instructions = update_user_vault_instructions(
        &config,
        &payer,
        &current_authority,
        &new_authority,
        &service_signer,
        params,
    )
    .await?;

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    let transaction = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[&payer, &current_authority, &service_signer, &new_authority],
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
