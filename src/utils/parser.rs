use crate::executor::binance::PlaceOrder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Deserialize, Serialize)]
pub struct EisenSwap {
    pub token_in: String,
    pub token_out: String,
    pub amount: f64,
}

#[derive(Debug)]
pub struct BinanceOrder {
    pub symbol: String,
    pub side: String, // "BUY" or "SELL"
    pub quantity: f64,
    pub price: Option<f64>,
}

pub fn extract_eisen_swaps(json_response: &Value) -> Vec<EisenSwap> {
    let mut swaps = Vec::new();

    // Try to extract swaps from the strategy JSON
    if let Some(exchanges) = json_response.get("exchanges").and_then(|e| e.as_array()) {
        for exchange in exchanges {
            if let Some(eisen) = exchange.get("eisen") {
                if let Some(swaps_array) = eisen.get("swaps").and_then(|s| s.as_array()) {
                    for swap in swaps_array {
                        if let (Some(token_in), Some(token_out), Some(amount)) = (
                            swap.get("token_in").and_then(|ti| ti.as_str()),
                            swap.get("token_out").and_then(|to| to.as_str()),
                            swap.get("amount").and_then(|a| a.as_str()),
                        ) {
                            // Convert amount to f64
                            let amount_f64 = amount.parse::<f64>().unwrap_or(0.0);

                            swaps.push(EisenSwap {
                                token_in: token_in.to_string(),
                                token_out: token_out.to_string(),
                                amount: amount_f64,
                            });
                        }
                    }
                }
            }
        }
    }

    swaps
}
pub fn extract_binance_place_order(json_response: &serde_json::Value) -> Vec<PlaceOrder> {
    let mut orders = Vec::new();

    // Try to extract orders from the strategy JSON
    if let Some(exchanges) = json_response.get("exchanges").and_then(|e| e.as_array()) {
        for exchange in exchanges {
            if let Some(binance_orders) = exchange.get("binance") {
                if let Some(exchange_orders) =
                    binance_orders.get("orders").and_then(|o| o.as_array())
                {

                    // Print the orders that will be processed
                    println!("Binance orders to be processed:");
                    for (i, order) in exchange_orders.iter().enumerate() {
                        let token = order.get("token").and_then(|t| t.as_str()).unwrap_or("unknown");
                        let side = order.get("side").and_then(|s| s.as_str()).unwrap_or("unknown");
                        let amount = order.get("amount").and_then(|a| a.as_str()).unwrap_or("unknown");
                        let price = order.get("price").and_then(|p| p.as_str()).unwrap_or("market price");
                        
                        println!("Order {}: {} {} {} at {}", 
                            i + 1, 
                            side, 
                            amount, 
                            token, 
                            price);
                    }
                    
                    if exchange_orders.is_empty() {
                        println!("No Binance orders to process");
                    }
                    for order in exchange_orders {
                        let symbol = order
                            .get("token")
                            .and_then(|s| s.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let symbol = format!("{}USDT", symbol);

                        let side_str = order
                            .get("side")
                            .and_then(|s| s.as_str())
                            .unwrap_or_default()
                            .to_uppercase();

                        // Convert string to OrderSide enum
                        let side = match side_str.as_str() {
                            "BUY" => crate::executor::binance::OrderSide::Buy,
                            "SELL" => crate::executor::binance::OrderSide::Sell,
                            _ => continue, // Skip invalid side
                        };

                        let order_type_str = order
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("MARKET")
                            .to_uppercase();

                        // Convert string to OrderType enum
                        let order_type = match order_type_str.as_str() {
                            "MARKET" => crate::executor::binance::OrderType::Market,
                            "LIMIT" => crate::executor::binance::OrderType::Limit,
                            _ => crate::executor::binance::OrderType::Market, // Default to market
                        };

                        // Parse quantity as Decimal
                        let quantity = order
                            .get("amount")
                            .and_then(|q| q.as_str())
                            .and_then(|q| rust_decimal::Decimal::from_str_exact(q).ok());

                        // Parse optional price field
                        let price = order
                            .get("price")
                            .and_then(|p| p.as_str())
                            .and_then(|p| rust_decimal::Decimal::from_str_exact(p).ok());

                        // Parse optional time_in_force field
                        let time_in_force =
                            order.get("timeInForce").and_then(|t| t.as_str()).map(|t| {
                                match t.to_uppercase().as_str() {
                                    "GTC" => crate::executor::binance::TimeInForce::Gtc,
                                    "IOC" => crate::executor::binance::TimeInForce::Ioc,
                                    "FOK" => crate::executor::binance::TimeInForce::Fok,
                                    "GTX" => crate::executor::binance::TimeInForce::Gtx,
                                    _ => crate::executor::binance::TimeInForce::Gtc,
                                }
                            });

                        // Parse optional reduce_only field
                        let reduce_only = order.get("reduceOnly").and_then(|r| r.as_bool());

                        // Parse optional close_position field
                        let close_position = order.get("closePosition").and_then(|c| c.as_bool());

                        // Validate required fields
                        orders.push(PlaceOrder {
                            symbol,
                            side,
                            position_side: None,
                            order_type,
                            reduce_only,
                            quantity,
                            price,
                            new_client_order_id: None,
                            stop_price: None,
                            close_position,
                            activation_price: None,
                            callback_rate: None,
                            time_in_force,
                            working_type: None,
                            price_protect: None,
                        });
                    }
                }
            }
        }
    }

    // Print orders for debugging
    println!("Extracted Binance orders:");
    for (i, order) in orders.iter().enumerate() {
        println!("Order {}: {:?}", i + 1, order);
    }
    
    if orders.is_empty() {
        println!("No Binance orders extracted");
    }
    orders
}
