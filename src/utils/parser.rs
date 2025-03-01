use crate::executor::binance::PlaceOrder;
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
pub struct EisenSwap {
    pub token_in: String,
    pub token_out: String,
    pub amount: f64,
}

pub fn extract_eisen_swaps(json_response: &serde_json::Value) -> Vec<EisenSwap> {
    let mut swaps = Vec::new();

    // Try to extract swaps from the strategy JSON
    if let Some(exchanges) = json_response.get("exchanges").and_then(|e| e.as_array()) {
        for exchange in exchanges {
            if let Some(target) = exchange.get("target").and_then(|t| t.as_str()) {
                if target.to_lowercase() == "eisen" {
                    if let Some(exchange_swaps) = exchange.get("swaps").and_then(|s| s.as_array()) {
                        for swap in exchange_swaps {
                            let token_in = swap
                                .get("token_in")
                                .and_then(|t| t.as_str())
                                .unwrap_or_default()
                                .to_string();

                            let token_out = swap
                                .get("token_out")
                                .and_then(|t| t.as_str())
                                .unwrap_or_default()
                                .to_string();

                            let amount = swap
                                .get("amount")
                                .and_then(|a| a.as_str())
                                .and_then(|a| a.parse::<f64>().ok())
                                .unwrap_or(0.0);

                            if !token_in.is_empty() && !token_out.is_empty() && amount > 0.0 {
                                swaps.push(EisenSwap {
                                    token_in,
                                    token_out,
                                    amount,
                                });
                            }
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
            if let Some(target) = exchange.get("target").and_then(|t| t.as_str()) {
                if target.to_lowercase() == "binance" {
                    if let Some(exchange_orders) = exchange.get("orders").and_then(|o| o.as_array())
                    {
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
                            let quantity_str = order
                                .get("quantity")
                                .and_then(|q| q.as_str())
                                .unwrap_or_default();

                            let quantity = if !quantity_str.is_empty() {
                                match rust_decimal::Decimal::from_str_exact(quantity_str) {
                                    Ok(q) => Some(q),
                                    Err(_) => None,
                                }
                            } else {
                                None
                            };

                            // Parse optional price field
                            let price = order
                                .get("price")
                                .and_then(|p| p.as_str())
                                .and_then(|p| rust_decimal::Decimal::from_str_exact(p).ok());

                            // Parse optional time_in_force field
                            let time_in_force = order
                                .get("timeInForce")
                                .and_then(|t| t.as_str())
                                .map(|t| match t.to_uppercase().as_str() {
                                    "GTC" => crate::executor::binance::TimeInForce::Gtc,
                                    "IOC" => crate::executor::binance::TimeInForce::Ioc,
                                    "FOK" => crate::executor::binance::TimeInForce::Fok,
                                    "GTX" => crate::executor::binance::TimeInForce::Gtx,
                                    _ => crate::executor::binance::TimeInForce::Gtc,
                                });

                            // Parse optional reduce_only field
                            let reduce_only = order.get("reduceOnly").and_then(|r| r.as_bool());

                            // Parse optional close_position field
                            let close_position =
                                order.get("closePosition").and_then(|c| c.as_bool());

                            // Validate required fields
                            if !symbol.is_empty() {
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
        }
    }

    orders
}
