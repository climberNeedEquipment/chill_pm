use crate::executor;
use crate::utils;
use crate::utils::parser::{extract_binance_place_order, extract_eisen_swaps};
use alloy::providers::Provider;
use std::error::Error;

pub async fn process_eisen_swaps(
    json_response: &serde_json::Value,
    provider: &dyn Provider,
    base_url: &str,
    chain_data: &executor::eisen::ChainData,
    wallet_addr: &alloy::primitives::Address,
) -> Result<(), Box<dyn Error>> {
    let swaps = extract_eisen_swaps(json_response);

    // Print the swaps that will be executed
    println!("Swaps to be executed:");
    for (i, swap) in swaps.iter().enumerate() {
        println!(
            "Swap {}: {} -> {} (amount: {})",
            i + 1,
            swap.token_in,
            swap.token_out,
            swap.amount
        );
    }

    if swaps.is_empty() {
        println!("No swaps to execute");
    }

    for swap in swaps {
        // Call the quote_and_send_tx function from executor/eisen
        let result = executor::eisen::quote_and_send_tx(
            provider,
            base_url,
            chain_data,
            &swap.token_in,
            &swap.token_out,
            swap.amount,
            wallet_addr,
            100, // Default slippage of 1% (100 basis points)
        )
        .await?;

        // Handle the result as needed
        println!("Eisen swap executed: {:?}", result);
    }

    Ok(())
}

// Function to process Binance positions from the strategy JSON
pub async fn process_binance_place_order(
    json_response: &serde_json::Value,
    binance_base_url: &str,
    binance_key: &utils::sign::BinanceKey,
) -> Result<(), Box<dyn Error>> {
    let positions = extract_binance_place_order(json_response);

    // Print the positions that will be executed
    println!("Positions to be executed:");
    for (i, position) in positions.iter().enumerate() {
        println!("Position {}: {:?}", i + 1, position);
    }

    if positions.is_empty() {
        println!("No positions to execute");
    }

    for position in positions {
        // Call the place_binance_order function
        let result = executor::binance::place_binance_order(
            binance_base_url,
            binance_key,
            &position.symbol, // Use token directly as symbol is constructed inside the function
            position.side,
            position.quantity,
            position.price,
            None, // No stop price for now
        )
        .await?;

        // Handle the result as needed
        println!("Binance position executed: {:?}", result);
    }

    Ok(())
}
