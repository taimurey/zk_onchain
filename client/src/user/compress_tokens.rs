use anchor_client::{Client, Cluster, Program};
use anchor_lang::{system_program, AnchorSerialize};
use anchor_spl::{associated_token::get_associated_token_address, token::spl_token};
use light_compressed_token::{
    get_token_pool_pda,
    process_transfer::{
        get_cpi_authority_pda, CompressedTokenInstructionDataTransfer,
        PackedTokenTransferOutputData,
    },
};
use light_sdk::{PROGRAM_ID_ACCOUNT_COMPRESSION, PROGRAM_ID_LIGHT_SYSTEM};
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
use std::sync::Arc;
use zk_onchain::{
    accounts as soda_accounts, instruction as soda_instructions, state::MINT_AUTHORITY,
    user::COMPRESSED_MINT_SEED,
};

use crate::{settings::config::load_cfg, utils::config::keypair_1};

use super::user_vault::get_program_addresses;

pub async fn compress_tokens_inx(
    program: Program<Arc<Keypair>>,
    service_signer: Pubkey,
    nonce: u16,
    amount: u64,
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

    let (_, registered_program_pda, account_compression_authority) = get_program_addresses()?;

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

    println!(
            "payer: {}\nservice_signer: {}\nauthority: {}\ncompressed_mint: {}\nassociated_vault_account: {}\nassociated_token_program: {}\ntoken_program: {}\nsystem_program: {}\nrent_program: {}",
            program.payer(),
            service_signer,
            pda_authority,
            derived_mint,
            associated_account,
            anchor_spl::associated_token::ID,
            spl_token::id(),
            system_program::ID,
            rent::id()
        );

    let compress_tokens = program
        .request()
        .accounts(soda_accounts::CompressTokens {
            payer: program.payer(),
            service_signer,
            authority: pda_authority,
            compress_token_account: associated_account,
            cpi_authority_pda: get_cpi_authority_pda().0,
            token_pool_pda: get_token_pool_pda(&derived_mint),
            registered_program_pda,
            account_compression_authority,
            noop_program: light_sdk::PROGRAM_ID_NOOP,
            light_system_program: PROGRAM_ID_LIGHT_SYSTEM,
            account_compression_program: PROGRAM_ID_ACCOUNT_COMPRESSION,
            compressed_token_program: light_compressed_token::id(),
            token_program: spl_token::id(),
            system_program: system_program::ID,
        })
        .args(soda_instructions::CompressTokens { inputs })
        .instructions()?;

    create_compressed_mint_ix.extend(compress_tokens);

    Ok(create_compressed_mint_ix)
}

pub async fn create_compress_tokens_inx(rpc_client: RpcClient) -> anyhow::Result<()> {
    let client_config = "client_config.ini";
    let config = load_cfg(&client_config.to_string()).unwrap();

    let payer = Arc::new(read_keypair_file(&config.payer_path).unwrap());
    let service_signer = keypair_1();

    let client = Client::new_with_options(
        Cluster::Custom(config.http_url.clone(), config.ws_url.clone()),
        payer.clone(),
        CommitmentConfig::processed(),
    );
    let program = client.program(zk_onchain::id())?;

    let compressed_mint_ix = compress_tokens_inx(program, service_signer.pubkey(), 4u16, 1).await?;

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

    println!("Calling without Signer");

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    let transaction = Transaction::new_signed_with_payer(
        &compressed_mint_ix,
        Some(&payer.pubkey()),
        &[&payer],
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

    Ok(())
}
