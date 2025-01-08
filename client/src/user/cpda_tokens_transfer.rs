use anchor_client::{Client, Cluster, Program};

use anchor_lang::system_program;
use anchor_spl::{associated_token::get_associated_token_address, token::spl_token};
use light_compressed_token::{get_token_pool_pda, process_transfer::get_cpi_authority_pda};
use light_sdk::merkle_context::AddressMerkleContext;

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    sysvar::rent,
    transaction::Transaction,
};
use std::{str::FromStr, sync::Arc, thread::sleep, time::Duration};
use zk_onchain::{
    accounts as soda_accounts, instruction as soda_instructions, state::MINT_AUTHORITY,
    user::COMPRESSED_MINT_SEED,
};

use crate::{
    settings::config::load_cfg,
    user::{
        compressed_transfer_ix::create_escrow_ix,
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
) -> anyhow::Result<(Vec<Instruction>, Pubkey)> {
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

    Ok((create_compressed_mint_ix, derived_mint))
}

pub async fn create_user_vaults_transfer(rpc_client: Arc<RpcClient>) -> anyhow::Result<()> {
    let client_config = "client_config.ini";
    let config = load_cfg(&client_config.to_string()).unwrap();

    let user_1 = Arc::new(Keypair::new());
    let user_1_pubkey = user_1.pubkey();

    let user_2 = Arc::new(Keypair::new());
    let user_2_pubkey = user_2.pubkey();

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

    let der_user_v2 = derive_user_vault(user_2_pubkey, address_merkle_context);
    let user_vault_2 = Pubkey::from(der_user_v2);

    let payer = Arc::new(read_keypair_file(&config.payer_path).unwrap());
    let service_signer = keypair_1();

    let client = Client::new_with_options(
        Cluster::Custom(config.http_url.clone(), config.ws_url.clone()),
        payer.clone(),
        CommitmentConfig::processed(),
    );
    let program = client.program(zk_onchain::id())?;
    use rand::Rng;
    let random_number = rand::thread_rng().gen_range(100..10000);
    let (compressed_mint_ix, mint) = user_vaults_transfer_ix(
        program,
        service_signer.pubkey(),
        random_number as u16,
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

    let (_, mut transfer_inx) = create_escrow_ix(
        program,
        &payer,
        user_1_pubkey,
        user_vault_1,
        user_vault_2,
        &mint,
    )
    .await?;

    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    transfer_inx.extend([
        ComputeBudgetInstruction::set_compute_unit_limit(1000000),
        ComputeBudgetInstruction::set_compute_unit_price(10000u64),
    ]);

    let transaction = Transaction::new_signed_with_payer(
        &transfer_inx,
        Some(&payer.pubkey()),
        &[&payer, &user_1],
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
