use anchor_spl::token::spl_token::instruction::initialize_mint2;
use anchor_spl::token::Mint;
use anyhow::Result;

use crate::settings::config::ClientConfig;
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction::create_account;
use solana_sdk::transaction::Transaction;

pub fn create_mint(config: &ClientConfig) -> Result<Pubkey> {
    let payer = read_keypair_file(&config.payer_path).unwrap();
    let rpc_client = RpcClient::new(config.http_url.clone());

    let token_mint = Keypair::new();

    let lamports = rpc_client.get_minimum_balance_for_rent_exemption(Mint::LEN)?;

    // Create account instruction with Token Program as owner
    let create_account = create_account(
        &payer.pubkey(),
        &token_mint.pubkey(),
        lamports,
        Mint::LEN as u64,
        &TOKEN_PROGRAM_ID, // This is the key change - setting Token Program as owner
    );

    // Initialize the mint
    let initialize = initialize_mint2(
        &TOKEN_PROGRAM_ID,
        &token_mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )?;

    let transaction = Transaction::new_signed_with_payer(
        &[create_account, initialize],
        Some(&payer.pubkey()),
        &[&token_mint, &payer],
        rpc_client.get_latest_blockhash()?,
    );

    rpc_client.send_and_confirm_transaction_with_spinner(&transaction)?;

    Ok(token_mint.pubkey())
}
