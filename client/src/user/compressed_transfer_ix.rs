use anchor_client::Program;

use anchor_lang::{prelude::AccountMeta, AnchorDeserialize};
use light_compressed_token::process_transfer::{
    get_cpi_authority_pda, transfer_sdk::create_inputs_and_remaining_accounts_checked,
    TokenTransferOutputData,
};
use light_sdk::legacy::CompressedAccountData;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        address::add_and_get_remaining_account_indices,
        compressed_account::{CompressedAccount, MerkleContext},
    },
};
use photon_api::{
    apis::{
        configuration::{ApiKey, Configuration},
        default_api::{
            get_compressed_account_post, get_compressed_accounts_by_owner_post,
            get_compressed_token_accounts_by_owner_post, get_validity_proof_post,
        },
    },
    models::{
        GetCompressedAccountPostRequest, GetCompressedAccountPostRequestParams,
        GetCompressedAccountsByOwnerPostRequest, GetCompressedAccountsByOwnerPostRequestParams,
        GetCompressedTokenAccountsByOwnerPostRequest,
        GetCompressedTokenAccountsByOwnerPostRequestParams, GetValidityProofPost200Response,
        GetValidityProofPostRequest, GetValidityProofPostRequestParams,
    },
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use std::{collections::HashMap, str::FromStr, sync::Arc};
use zk_onchain::{
    accounts as soda_accounts, instruction as soda_instructions, vaults::UserVaultState,
};

use crate::utils::vectorizer::vec_to_array;

#[derive(Debug, Clone)]
pub struct CreateCompressedTransferParamInstructions<'a> {
    pub signer: &'a Pubkey,
    pub input_merkle_context: &'a [MerkleContext],
    pub output_compressed_account_merkle_tree_pubkeys: &'a [Pubkey],
    pub output_compressed_accounts: &'a [TokenTransferOutputData],
    pub root_indices: &'a [u16],
    pub proof: &'a Option<CompressedProof>,
    pub input_token_data: &'a [light_compressed_token::token_data::TokenData],
    pub input_compressed_accounts: &'a [CompressedAccount],
    pub mint: &'a Pubkey,
    pub cpi_context_account: &'a Pubkey,
}

pub async fn create_escrow_ix(
    program: Program<Arc<Keypair>>,
    payer: &Keypair,
    source_pda_owner: Pubkey,
    source_pda: Pubkey,
    destination_pda: Pubkey,
    mint: &Pubkey,
) -> anyhow::Result<(anchor_lang::prelude::Pubkey, Vec<Instruction>)> {
    println!(
        "payer: {},
        source_pda: {}
        destination_pda: {}
        mint: {}",
        payer.pubkey(),
        source_pda,
        destination_pda,
        mint
    );

    let payer_pubkey = payer.pubkey();

    let address_merkle_tree_queue_pubkey =
        Pubkey::from_str("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F").unwrap();
    let address_merkle_tree_pubkey =
        Pubkey::from_str("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2").unwrap();
    let merkle_tree_pubkey =
        Pubkey::from_str("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT").unwrap();
    let nullifier_queue_pubkey =
        Pubkey::from_str("nfq1NvQDJ2GEgnS8zt9prAe8rjjpAW1zFkrvZoBR148").unwrap();

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

    let mut item_account = photon_api::models::Account::default();
    if let Some(compressed_account) = compressed_accounts.result {
        item_account = compressed_account.value.items[0].clone();
        let compressed_data = item_account.data.clone().unwrap().data;

        // First decode the base58 string into a Vec<u8>
        let decoded_bytes = base64::decode(compressed_data.clone())?;
        // Create a mutable slice reference
        let user_vault_state = UserVaultState::try_from_slice(&decoded_bytes)?;

        println!("User Vault: {user_vault_state:#?}");
    }

    let source_account = get_compressed_account_post(
        &rpc_config,
        GetCompressedAccountPostRequest {
            params: Box::new(GetCompressedAccountPostRequestParams {
                address: Some(Some(source_pda.to_string())),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;

    let destination_account = get_compressed_account_post(
        &rpc_config,
        GetCompressedAccountPostRequest {
            params: Box::new(GetCompressedAccountPostRequestParams {
                address: Some(Some(destination_pda.to_string())),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;

    let proof_result = get_validity_proof_post(
        &rpc_config,
        GetValidityProofPostRequest {
            params: Box::new(GetValidityProofPostRequestParams {
                hashes: Some(vec![
                    source_account.result.clone().unwrap().value.unwrap().hash,
                    destination_account
                        .result
                        .clone()
                        .unwrap()
                        .value
                        .unwrap()
                        .hash,
                ]),
                new_addresses: None,
                new_addresses_with_trees: None,
            }),
            ..Default::default()
        },
    )
    .await?;

    let cpi_context_account_pubkey =
        Pubkey::from_str("cpi1uHzrEhBG733DoEJNgHCyRS3XmmyVNZx5fonubE4").unwrap();

    let (derived_proof, root_indices) = get_legacy_proof(proof_result.clone()).unwrap();

    let source_compressed_data = source_account
        .result
        .as_ref()
        .and_then(|r| r.value.as_ref())
        .and_then(|v| v.data.as_ref())
        .map(|data| -> anyhow::Result<CompressedAccountData> {
            Ok(CompressedAccountData {
                discriminator: data.discriminator.to_le_bytes(),
                data: base64::decode(&data.data)?,
                data_hash: bs58::decode(&data.data_hash)
                    .into_vec()?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid hash length"))?,
            })
        })
        .transpose()?;

    let destination_compressed_data = destination_account
        .result
        .as_ref()
        .and_then(|r| r.value.as_ref())
        .and_then(|v| v.data.as_ref())
        .map(|data| -> anyhow::Result<CompressedAccountData> {
            Ok(CompressedAccountData {
                discriminator: data.discriminator.to_le_bytes(),
                data: base64::decode(&data.data)?,
                data_hash: bs58::decode(&data.data_hash)
                    .into_vec()?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid hash length"))?,
            })
        })
        .transpose()?;

    // GET TokenData
    let compressed_token_accounts = get_compressed_token_accounts_by_owner_post(
        &rpc_config,
        GetCompressedTokenAccountsByOwnerPostRequest {
            params: Box::new(GetCompressedTokenAccountsByOwnerPostRequestParams {
                owner: source_pda.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await;

    let accounts_clone = compressed_token_accounts.unwrap().clone();
    let accounts_result = accounts_clone.result.unwrap();
    let input_token_result = accounts_result
        .value
        .items
        .iter()
        .find(|x| x.token_data.owner.clone() == source_pda.to_string());

    println!("input token vault: {:#?}", input_token_result);

    // GET TokenData
    let compressed_token_accounts = get_compressed_token_accounts_by_owner_post(
        &rpc_config,
        GetCompressedTokenAccountsByOwnerPostRequest {
            params: Box::new(GetCompressedTokenAccountsByOwnerPostRequestParams {
                owner: destination_pda.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await;

    let accounts_clone = compressed_token_accounts.unwrap().clone();
    let accounts_result = accounts_clone.result.unwrap();
    let output_token_result = accounts_result
        .value
        .items
        .iter()
        .find(|x| x.token_data.owner.clone() == destination_pda.to_string());

    println!("output token vault: {:#?}", output_token_result);

    let input_token_data = light_compressed_token::TokenData {
        amount: input_token_result.unwrap().token_data.amount as u64,
        mint: Pubkey::from_str(&input_token_result.unwrap().token_data.mint).unwrap(),
        owner: Pubkey::from_str(&input_token_result.unwrap().token_data.owner).unwrap(),
        delegate: None,
        state: match input_token_result.unwrap().token_data.state {
            photon_api::models::AccountState::Initialized => {
                light_compressed_token::token_data::AccountState::Initialized
            }
            photon_api::models::AccountState::Frozen => {
                light_compressed_token::token_data::AccountState::Frozen
            }
        },
        tlv: None,
    };

    let transfer_amount = 10u64;
    let input_amount = 100000u64;
    let change_amount = input_amount - transfer_amount;

    // First the change token data
    let change_token_data = TokenTransferOutputData {
        owner: source_pda,
        amount: change_amount,
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };

    let destination_token_data = TokenTransferOutputData {
        owner: destination_pda,
        amount: transfer_amount,
        lamports: None,
        merkle_tree: merkle_tree_pubkey,
    };

    let create_ix_inputs = CreateCompressedTransferParamInstructions {
        input_token_data: &[input_token_data],
        signer: &source_pda,
        input_merkle_context: &[
            MerkleContext {
                leaf_index: proof_result.result.clone().unwrap().value.leaf_indices[0] as u32,
                merkle_tree_pubkey,
                nullifier_queue_pubkey,
                queue_index: None,
            },
            MerkleContext {
                leaf_index: proof_result.result.clone().unwrap().value.leaf_indices[1] as u32,
                merkle_tree_pubkey,
                nullifier_queue_pubkey,
                queue_index: None,
            },
        ],
        output_compressed_account_merkle_tree_pubkeys: &[merkle_tree_pubkey, merkle_tree_pubkey],
        output_compressed_accounts: &[change_token_data, destination_token_data],
        root_indices: &[root_indices],
        proof: &Some(derived_proof),
        mint,
        cpi_context_account: &cpi_context_account_pubkey,
        input_compressed_accounts: &[
            light_system_program::sdk::compressed_account::CompressedAccount {
                owner: zk_onchain::ID,
                lamports: 0,
                address: Some(source_pda.to_bytes()),
                data: source_compressed_data,
            },
            light_system_program::sdk::compressed_account::CompressedAccount {
                owner: zk_onchain::ID,
                lamports: 0,
                address: Some(destination_pda.to_bytes()),
                data: destination_compressed_data,
            },
        ],
    };

    let instruction = create_escrow_instruction(
        program,
        create_ix_inputs.clone(),
        payer_pubkey,
        source_pda_owner,
        source_pda,
        destination_pda,
    );

    Ok((payer_pubkey, instruction))
}

pub fn create_escrow_instruction(
    program: Program<Arc<Keypair>>,
    input_params: CreateCompressedTransferParamInstructions,
    payer: Pubkey,
    source_pda_owner: Pubkey,
    source_pda: Pubkey,
    destination_pda: Pubkey,
) -> Vec<Instruction> {
    let (mut remaining_accounts, inputs) = create_inputs_and_remaining_accounts_checked(
        input_params.input_token_data,
        input_params.input_compressed_accounts,
        input_params.input_merkle_context,
        None,
        input_params.output_compressed_accounts,
        input_params.root_indices,
        input_params.proof,
        *input_params.mint,
        input_params.signer,
        false,
        None,
        None,
        None,
    )
    .unwrap();

    let merkle_tree_indices = add_and_get_remaining_account_indices(
        input_params.output_compressed_account_merkle_tree_pubkeys,
        &mut remaining_accounts,
    );

    println!("Merkle Tree: {:?}", merkle_tree_indices);

    let cpi_context_account_index: u8 = match remaining_accounts
        .get(input_params.cpi_context_account)
    {
        Some(entry) => (*entry).try_into().unwrap(),
        None => {
            remaining_accounts.insert(*input_params.cpi_context_account, remaining_accounts.len());
            (remaining_accounts.len() - 1) as u8
        }
    };
    let instruction_data = soda_instructions::TransferCompressedTokensWithPda {
        proof: input_params.proof.clone().unwrap(),
        mint: *input_params.mint,
        amount: 10u64,
        input_token_data_with_context: inputs.input_token_data_with_context,
        output_state_merkle_tree_account_indices: merkle_tree_indices,
        cpi_context: light_sdk::legacy::CompressedCpiContext {
            set_context: false,
            first_set_context: true,
            cpi_context_account_index,
        },
    };

    let registered_program_pda = Pubkey::find_program_address(
        &[light_system_program::ID.to_bytes().as_slice()],
        &account_compression::ID,
    )
    .0;
    let compressed_token_cpi_authority_pda = get_cpi_authority_pda().0;
    let account_compression_authority =
        light_system_program::utils::get_cpi_authority_pda(&light_system_program::ID);
    let cpi_authority_pda = light_sdk::utils::get_cpi_authority_pda(&zk_onchain::id());

    let mut instructions = program
        .request()
        .accounts(soda_accounts::TransferCompressedTokensWithPda {
            signer: payer,
            source_pda_owner,
            source_pda,
            destination_pda,
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            compressed_token_program: light_compressed_token::ID,
            light_system_program: light_system_program::ID,
            account_compression_program: account_compression::ID,
            registered_program_pda,
            compressed_token_cpi_authority_pda,
            account_compression_authority,
            self_program: zk_onchain::id(),
            system_program: solana_sdk::system_program::id(),
            cpi_context_account: *input_params.cpi_context_account,
            cpi_authority_pda: get_cpi_authority_pda().0,
        })
        .args(instruction_data)
        .instructions()
        .unwrap();

    instructions[0]
        .accounts
        .extend(convert_remaining_accounts_to_metas(remaining_accounts));

    instructions
}

fn convert_remaining_accounts_to_metas(
    remaining_accounts: HashMap<Pubkey, usize>,
) -> Vec<AccountMeta> {
    // Convert the HashMap entries into a vector of tuples for sorting
    let mut accounts: Vec<_> = remaining_accounts.into_iter().collect();

    // Sort by the index (usize value) to maintain correct order
    accounts.sort_by_key(|(_, idx)| *idx);

    // Convert to AccountMetas
    accounts
        .into_iter()
        .map(|(pubkey, _)| AccountMeta::new(pubkey, false)) // Assuming these are read-only accounts
        .collect()
}

pub fn get_legacy_proof(
    rpc_result: GetValidityProofPost200Response,
) -> anyhow::Result<(CompressedProof, u16)> {
    let mut compressed_proof = CompressedProof {
        a: [0; 32],
        b: [0; 64],
        c: [0; 32],
    };
    let mut root_indices: u16 = 0;

    if let Some(result_box) = rpc_result.result {
        let result = *result_box.value.compressed_proof;

        let a_array: [u8; 32] = vec_to_array(result.a, "a").unwrap();
        let b_array: [u8; 64] = vec_to_array(result.b, "b").unwrap();
        let c_array: [u8; 32] = vec_to_array(result.c, "c").unwrap();

        root_indices = result_box.value.root_indices[0] as u16;

        compressed_proof = CompressedProof {
            a: a_array, //  a (32 bytes)
            b: b_array, //  b (64 bytes)
            c: c_array, //  c (32 bytes)
        };
    } else {
        // Handle the case where `result` is None
        eprintln!("Error: rpc_result.result is None");

        compressed_proof = CompressedProof {
            a: [0; 32],
            b: [0; 64],
            c: [0; 32],
        };
    }

    Ok((compressed_proof, root_indices))
}
