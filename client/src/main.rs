#![allow(dead_code)]

use anyhow::{format_err, Result};
use raydium_library::amm;
use std::str::FromStr;

use solana_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::Signer, transaction::Transaction};

fn send_swap_tx() -> Result<()> {
    println!("calling send_swap_tx...");
    // config params
    let wallet_file_path = "id.json";
    let cluster_url = "YOUR RPC";
    let amm_program = Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")?;
    let amm_pool_id = Pubkey::from_str("4YcwqxR7eoH29rF2G7U9YmtzCt1n7tyHcywg3Emif85s")?;
    let input_token_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v")?;
    let output_token_mint = Pubkey::from_str("F9CpWoyeBJfoRB8f2pBe2ZNPbPsEE76mWZWme3StsvHK")?;
    let slippage_bps = 50u64; // 0.5%
    let amount_specified = 100000u64;
    let swap_base_in = false;

    let client = RpcClient::new(cluster_url.to_string());
    let wallet = solana_sdk::signature::read_keypair_file(wallet_file_path)
        .map_err(|_| format_err!("failed to read keypair from {}", wallet_file_path))?;
    println!("wallet: {}", wallet.pubkey());
    // load amm keys
    let amm_keys = raydium_library::amm::utils::load_amm_keys(&client, &amm_program, &amm_pool_id)?;
    // load market keys
    let market_keys = raydium_library::amm::openbook::get_keys_for_market(
        &client,
        &amm_keys.market_program,
        &amm_keys.market,
    )?;
    // calculate amm pool vault with load data at the same time or use simulate to calculate
    let result = raydium_library::amm::calculate_pool_vault_amounts(
        &client,
        &amm_program,
        &amm_pool_id,
        &amm_keys,
        &market_keys,
        amm::utils::CalculateMethod::Simulate(wallet.pubkey()),
    )?;
    let direction = if input_token_mint == amm_keys.amm_coin_mint
        && output_token_mint == amm_keys.amm_pc_mint
    {
        amm::utils::SwapDirection::Coin2PC
    } else {
        amm::utils::SwapDirection::PC2Coin
    };
    let other_amount_threshold = raydium_library::amm::swap_with_slippage(
        result.pool_pc_vault_amount,
        result.pool_coin_vault_amount,
        result.swap_fee_numerator,
        result.swap_fee_denominator,
        direction,
        amount_specified,
        swap_base_in,
        slippage_bps,
    )?;
    println!(
        "amount_specified:{}, other_amount_threshold:{}",
        amount_specified, other_amount_threshold
    );

    let build_swap_instruction = raydium_library::amm::instructions::swap(
        &amm_program,
        &amm_keys,
        &market_keys,
        &wallet.pubkey(),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &input_token_mint,
        ),
        &spl_associated_token_account::get_associated_token_address(
            &wallet.pubkey(),
            &output_token_mint,
        ),
        amount_specified,
        other_amount_threshold,
        swap_base_in,
    )?;

    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &vec![build_swap_instruction],
        Some(&wallet.pubkey()),
        &vec![&wallet],
        client.get_latest_blockhash()?,
    );
    let sig = raydium_library::common::rpc::send_txn(&client, &txn, true)?;
    println!("sig:{:#?}", sig);
    Ok(())
}

fn main() -> Result<()> {
    send_swap_tx()?;
    Ok(())
}
