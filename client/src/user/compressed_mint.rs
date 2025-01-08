use anchor_client::{Client, Cluster, Program};
use anchor_lang::system_program;
use anchor_spl::token::spl_token;
use light_compressed_token::{get_token_pool_pda, process_transfer::get_cpi_authority_pda};
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

use crate::{
    settings::config::load_cfg,
    utils::{
        config::keypair_1,
        pinata_service::{json_metadata_ipfs, pinata_image_ipfs, JsonMetaData},
    },
};

pub async fn create_compressed_mint_inx(
    program: Program<Arc<Keypair>>,
    service_signer: Pubkey,
    nonce: u16,
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

    let image_metadata = pinata_image_ipfs().await.unwrap();
    let uri = json_metadata_ipfs(JsonMetaData {
        name: "cMINT".into(),
        symbol: "$cMINT".into(),
        image: image_metadata,
        description: None,
    })
    .await
    .unwrap();

    // authority
    let (pda_authority, _) =
        Pubkey::find_program_address(&[&MINT_AUTHORITY.as_bytes()], &zk_onchain::ID);

    let (metadata_account, _) = Pubkey::find_program_address(
        &[
            "metadata".as_bytes(),
            mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID.as_ref(),
            &derived_mint.to_bytes(),
        ],
        &mpl_token_metadata::programs::MPL_TOKEN_METADATA_ID,
    );

    Ok(program
        .request()
        .accounts(soda_accounts::CreateCompressedMint {
            payer: program.payer(),
            service_signer,
            authority: pda_authority,
            cpi_authority_pda: get_cpi_authority_pda().0,
            token_pool_pda: get_token_pool_pda(&derived_mint),
            compressed_mint: derived_mint,
            metadata_account: metadata_account,
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
            uri,
            nonce,
        })
        .instructions()?)
}

pub async fn create_compressed_mint(rpc_client: RpcClient) -> anyhow::Result<()> {
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

    let compressed_mint_ix =
        create_compressed_mint_inx(program, service_signer.pubkey(), 3u16).await?;

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
