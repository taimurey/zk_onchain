use anchor_client::{Client, Cluster, Program};

use anchor_lang::{system_program, AnchorSerialize, Id};
use anchor_spl::{
    associated_token::{self, get_associated_token_address},
    token::spl_token,
};
use light_compressed_token::{
    get_token_pool_pda,
    process_transfer::{
        get_cpi_authority_pda, CompressedTokenInstructionDataTransfer,
        PackedTokenTransferOutputData,
    },
    program::LightCompressedToken,
};
use light_sdk::merkle_context::AddressMerkleContext;
use light_system_program::{
    invoke::processor::CompressedProof,
    sdk::{
        compressed_account::{CompressedAccount, MerkleContext},
        invoke::create_invoke_instruction_data_and_remaining_accounts,
    },
    InstructionDataInvoke, NewAddressParams,
};
use photon_api::{
    apis::{
        configuration::{ApiKey, Configuration},
        default_api::get_compressed_accounts_by_owner_post,
    },
    models::{
        GetCompressedAccountsByOwnerPostRequest, GetCompressedAccountsByOwnerPostRequestParams,
    },
};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    sysvar::rent,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::time::sleep;
use zk_onchain::{
    accounts as soda_accounts,
    // compressed_transfers::sdk::{get_token_owner_pda, CreateCompressedPdaEscrowInstructionInputs},
    instruction as soda_instructions,
    state::MINT_AUTHORITY,
    user::COMPRESSED_MINT_SEED,
};

use crate::{
    settings::config::load_cfg,
    user::{
        compressed_vault_bump::derive_user_vault_with_bump,
        user_vault::{derive_user_vault, initialize_user_vault},
    },
    utils::config::keypair_1,
};

pub async fn user_vaults_transfer_ix(
    program: Program<Arc<Keypair>>,
    service_signer: Pubkey,
    nonce: u16,
    amount: u64,
    rpc_client: &RpcClient,
    user_1: Arc<Keypair>,
    user_2: Arc<Keypair>,
    user_1_vault: Pubkey,
    user_2_vault: Pubkey,
) -> anyhow::Result<Vec<Instruction>> {
    // mint
    let (derived_mint, _) = Pubkey::find_program_address(
        &[
            &COMPRESSED_MINT_SEED.as_bytes(),
            &program.payer().to_bytes(),
            &nonce.to_be_bytes(),
        ],
        &zk_onchain::ID,
    );

    initialize_user_vault(rpc_client, Some(user_1), Some(user_1_vault)).await?;
    initialize_user_vault(rpc_client, Some(user_2), Some(user_2_vault)).await?;

    let address_merkle_tree_queue_pubkey =
        Pubkey::from_str("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F")?;

    let address_merkle_tree_pubkey =
        Pubkey::from_str("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2")?;

    let merkle_tree_pubkey = Pubkey::from_str("smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT")?;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey: address_merkle_tree_queue_pubkey,
    };

    // authority
    let (pda_authority, _) =
        Pubkey::find_program_address(&[&MINT_AUTHORITY.as_bytes()], &zk_onchain::ID);

    let associated_account = get_associated_token_address(&pda_authority, &derived_mint);

    let (metadata_account, _) = Pubkey::find_program_address(
        &[
            "metadata".as_bytes(),
            mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID.as_ref(),
            &derived_mint.to_bytes(),
        ],
        &mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID,
    );

    let struct_data = CompressedTokenInstructionDataTransfer {
        proof: None, // Correctly initialized as None
        mint: derived_mint,
        delegated_transfer: None,
        cpi_context: None,
        lamports_change_account_merkle_tree_index: None,
        is_compress: true,
        output_compressed_accounts: vec![PackedTokenTransferOutputData {
            owner: associated_account,
            amount: 1,
            lamports: None, // Ensure this matches `Option<u64>` and is serialized correctly
            merkle_tree_index: 0,
            tlv: None,
        }],
        input_token_data_with_context: Vec::new(),
        compress_or_decompress_amount: Some(1), // Correctly initialized as Some
    };

    let mut inputs = Vec::new();
    CompressedTokenInstructionDataTransfer::serialize(&struct_data, &mut inputs)
        .expect("Serialization failed");

    println!("Serialized struct data: {:?}", inputs);

    let mut create_compressed_mint_ix = program
        .request()
        .accounts(soda_accounts::CreateCompressedMint {
            payer: program.payer(),
            service_signer,
            authority: pda_authority,
            cpi_authority_pda: get_cpi_authority_pda().0,
            token_pool_pda: get_token_pool_pda(&derived_mint),
            compressed_mint: derived_mint,
            metadata_account,
            compressed_token_program: light_compressed_token::id(),
            token_program: spl_token::id(),
            system_program: system_program::ID,
            rent_program: rent::id(),
            mpl_token_metadata: mpl_token_metadata::ID,
        })
        .args(soda_instructions::CreateCompressedMint {
            name: "cMINT".into(),
            symbol: "$cMINT".into(),
            decimals: 6,
            uri: "URI".into(),
            nonce,
        })
        .instructions()?;

    let minting_ix = program
        .request()
        .accounts(soda_accounts::MintTokens {
            payer: program.payer(),
            service_signer,
            cpi_authority_pda: get_cpi_authority_pda().0,
            authority: pda_authority,
            mint: derived_mint,
            token_pool_pda: get_token_pool_pda(&derived_mint),
            associated_token_program: anchor_spl::associated_token::ID,
            token_program: spl_token::id(),
            system_program: system_program::ID,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_authority: light_system_program::utils::get_cpi_authority_pda(
                &light_system_program::ID,
            ),
            light_compressed_token: light_compressed_token::ID,
            account_compression_program: account_compression::ID,
            light_system_program: light_system_program::id(),
            merkle_tree: merkle_tree_pubkey,
            rent_program: rent::id(),
            sol_pool_pda: None,
        })
        .args(soda_instructions::MintTokens {
            public_keys: vec![user_1_vault],
            amounts: vec![100000u64],
            lamports: None,
        })
        .instructions()?;

    create_compressed_mint_ix.extend(minting_ix);

    Ok(create_compressed_mint_ix)
}

pub async fn create_wallets_transfer(rpc_client: Arc<RpcClient>) -> anyhow::Result<()> {
    let client_config = "client_config.ini";
    let config = load_cfg(&client_config.to_string()).unwrap();

    let user_1 = Arc::new(Keypair::new());
    let user_1_pubkey = user_1.pubkey();

    println!("{}", user_1_pubkey);
    let user_2 = Arc::new(Keypair::new());
    let user_2_pubkey = user_2.pubkey();
    println!("{}", user_2_pubkey);

    let address_merkle_tree_queue_pubkey =
        Pubkey::from_str("aq1S9z4reTSQAdgWHGD2zDaS39sjGrAxbR31vxJ2F4F")?;

    let address_merkle_tree_pubkey =
        Pubkey::from_str("amt1Ayt45jfbdw5YSo7iz6WZxUmnZsQTYXy82hVwyC2")?;

    let address_merkle_context = AddressMerkleContext {
        address_merkle_tree_pubkey,
        address_queue_pubkey: address_merkle_tree_queue_pubkey,
    };

    let der_user_v1 = derive_user_vault(user_1_pubkey, address_merkle_context);
    let user_vault_1 = Pubkey::from(der_user_v1);

    let (user_vault_1_bumped, bump) =
        derive_user_vault_with_bump(user_1_pubkey, address_merkle_context);

    println!("1: {user_vault_1}");
    println!(
        "Derived 1: {}\n bump:{bump}",
        Pubkey::from(user_vault_1_bumped)
    );

    let der_user_v2 = derive_user_vault(user_2_pubkey, address_merkle_context);
    let user_vault_2 = Pubkey::from(der_user_v2);

    println!("derived 2: {user_vault_2}");

    let payer = Arc::new(read_keypair_file(&config.payer_path).unwrap());
    let service_signer = keypair_1();
    let recipient = Keypair::new();

    let client = Client::new_with_options(
        Cluster::Custom(config.http_url.clone(), config.ws_url.clone()),
        payer.clone(),
        CommitmentConfig::processed(),
    );
    let program = client.program(zk_onchain::id())?;

    let compressed_mint_ix = user_vaults_transfer_ix(
        program,
        service_signer.pubkey(),
        43u16,
        1,
        rpc_client.as_ref(),
        user_1.clone(),
        user_2,
        user_vault_1,
        user_vault_2,
    )
    .await?;

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    let transaction = Transaction::new_signed_with_payer(
        &compressed_mint_ix,
        Some(&payer.pubkey()),
        &[&payer, &service_signer],
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
    let program = client.program(zk_onchain::id())?;
    let transfer_inx =
        compressed_transfer(program, &user_1_pubkey, user_vault_1, recipient.pubkey()).await;

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    let transaction = Transaction::new_signed_with_payer(
        &transfer_inx,
        Some(&payer.pubkey()),
        &[&payer, &user_1],
        recent_blockhash,
    );

    rpc_client
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

pub async fn compressed_transfer(
    program: Program<Arc<Keypair>>,
    current_authority: &Pubkey,
    user_1_vault: Pubkey,
    recipient: Pubkey,
) -> Vec<Instruction> {
    sleep(Duration::from_secs(10)).await;
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
                owner: zk_onchain::id().to_string(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    if let Some(compressed_account) = compressed_accounts.result {
        println!("{compressed_account:?}");
    }

    let input_compressed_accounts = vec![
        CompressedAccount {
            lamports: 100,
            owner: user_1_vault,
            address: None,
            data: None,
        },
        CompressedAccount {
            lamports: 100,
            owner: user_1_vault,
            address: None,
            data: None,
        },
    ];

    let output_compressed_accounts = vec![
        CompressedAccount {
            lamports: 50,
            owner: user_1_vault,
            address: None,
            data: None,
        },
        CompressedAccount {
            lamports: 100,
            owner: recipient,
            address: None,
            data: None,
        },
    ];
    let merkle_tree_indices = vec![0, 2];
    let merkle_tree_pubkey = Keypair::new().pubkey();
    let merkle_tree_pubkey_1 = Keypair::new().pubkey();

    let nullifier_array_pubkey = Keypair::new().pubkey();
    let input_merkle_context = vec![
        MerkleContext {
            merkle_tree_pubkey,
            nullifier_queue_pubkey: nullifier_array_pubkey,
            leaf_index: 0,
            queue_index: None,
        },
        MerkleContext {
            merkle_tree_pubkey,
            nullifier_queue_pubkey: nullifier_array_pubkey,
            leaf_index: 1,
            queue_index: None,
        },
    ];

    let output_compressed_account_merkle_tree_pubkeys =
        vec![merkle_tree_pubkey, merkle_tree_pubkey_1];
    let input_root_indices = vec![0, 1];
    let proof = CompressedProof {
        a: [0u8; 32],
        b: [1u8; 64],
        c: [0u8; 32],
    };

    let payer = program.payer();

    create_invoke_instruction(
        program,
        &payer,
        &payer,
        current_authority,
        user_1_vault,
        &input_compressed_accounts[..],
        &output_compressed_accounts[..],
        &input_merkle_context[..],
        &output_compressed_account_merkle_tree_pubkeys,
        &input_root_indices.clone(),
        Vec::<NewAddressParams>::new().as_slice(),
        Some(proof.clone()),
        Some(200),
        true,
        None,
        true,
    )
}

pub fn create_invoke_instruction(
    program: Program<Arc<Keypair>>,
    fee_payer: &Pubkey,
    service_signer: &Pubkey,
    current_authority: &Pubkey,
    user_1_vault: Pubkey,
    input_compressed_accounts: &[CompressedAccount],
    output_compressed_accounts: &[CompressedAccount],
    merkle_context: &[MerkleContext],
    output_compressed_account_merkle_tree_pubkeys: &[Pubkey],
    input_root_indices: &[u16],
    new_address_params: &[NewAddressParams],
    proof: Option<CompressedProof>,
    compress_or_decompress_lamports: Option<u64>,
    is_compress: bool,
    decompression_recipient: Option<Pubkey>,
    sort: bool,
) -> Vec<Instruction> {
    let (remaining_accounts, mut inputs_struct) =
        create_invoke_instruction_data_and_remaining_accounts(
            new_address_params,
            merkle_context,
            input_compressed_accounts,
            input_root_indices,
            output_compressed_account_merkle_tree_pubkeys,
            output_compressed_accounts,
            proof,
            compress_or_decompress_lamports,
            is_compress,
        );
    if sort {
        inputs_struct
            .output_compressed_accounts
            .sort_by(|a, b| a.merkle_tree_index.cmp(&b.merkle_tree_index));
    }
    let mut inputs = Vec::new();

    InstructionDataInvoke::serialize(&inputs_struct, &mut inputs).unwrap();

    // program.request().accounts(soda_accounts::EscrowCompressedTokensWithCompressedPda{
    //     signer: *fee_payer,
    //     token_owner_pda:
    // })
    let mut instruction = program
        .request()
        .accounts(soda_accounts::TransferCompressedTokensWallet {
            payer: *fee_payer,
            registered_program_pda: light_system_program::utils::get_registered_program_pda(
                &light_system_program::ID,
            ),
            noop_program: Pubkey::new_from_array(
                account_compression::utils::constants::NOOP_PUBKEY,
            ),
            account_compression_program: account_compression::ID,
            account_compression_authority: get_cpi_authority_pda().0,
            service_signer: *service_signer,
            current_authority: *current_authority,
            user_vault: user_1_vault,
            self_program: zk_onchain::id(),
            light_compressed_token: LightCompressedToken::id(),
            system_program: solana_sdk::system_program::ID,
            associated_token_program: associated_token::ID,
            rent_program: rent::id(),
        })
        .args(soda_instructions::TransferCompressedTokens {
            transfer_inputs: inputs,
        })
        .instructions()
        .unwrap();

    instruction[0].accounts.extend(remaining_accounts);

    instruction
}
