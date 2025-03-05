use crate::agent::Strategy;
use crate::executor;
use crate::executor::eisen::ChainData;
use crate::utils;
use crate::utils::parser::extract_binance_place_order;
use alloy::providers::Provider;
use std::error::Error;

pub async fn process_eisen_swaps(
    strategy: &Strategy,
    provider: &Box<dyn Provider>,
    base_url: &str,
    chain_data: &ChainData,
    wallet_address: &String,
) -> Result<(), Box<dyn Error>> {
    let wallet_addr = wallet_address.parse::<alloy::primitives::Address>()?;

    if strategy.exchanges.eisen.swaps.is_none() {
        println!("No swaps to execute");
        return Ok(());
    }

    let swaps = strategy.exchanges.eisen.swaps.as_ref().unwrap(); 

    for (i, swap) in swaps.iter().enumerate() {
        println!(
            "Swap {}: {} -> {} (amount: {})",
            i + 1,
            swap.token_in,
            swap.token_out,
            swap.amount
        );
    }

    for swap in swaps {
        // Call the quote_and_send_tx function from executor/eisen
        let result = executor::eisen::quote_and_send_tx(
            provider.as_ref(),
            base_url,
            chain_data,
            &swap.token_in,
            &swap.token_out,
            swap.amount.parse::<f64>()?,
            &wallet_addr,
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
    strategy: &Strategy,
    binance_base_url: &str,
    binance_key: &utils::sign::BinanceKey,
) -> Result<(), Box<dyn Error>> {
    let binance_orders = extract_binance_place_order(strategy);

    if binance_orders.is_empty() {
        println!("No positions to execute");
    }

    for order in binance_orders {
        // Call the place_binance_order function
        let result = executor::binance::place_binance_order(
            binance_base_url,
            binance_key,
            &order.symbol, // Use token directly as symbol is constructed inside the function
            order.side,
            order.quantity,
            order.price,
            None, // No stop price for now
        )
        .await?;

        // Handle the result as needed
        println!("Binance position executed: {:?}", result);
    }

    Ok(())
}
