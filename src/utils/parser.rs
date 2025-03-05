use crate::agent::Strategy;
use crate::executor::binance::PlaceOrder;

pub fn extract_binance_place_order(strategy: &Strategy) -> Vec<PlaceOrder> {
    let mut orders = Vec::new();

    let binance_orders = &strategy.exchanges.binance.orders;

    if binance_orders.is_none() {
        println!("No Binance orders found");
        return orders;
    }

    let binance_orders = binance_orders.as_ref().unwrap();

    for order in binance_orders {
        println!("Order: {:?}", order);
        let symbol = format!("{}USDT", order.token.to_uppercase());

        // Convert string to OrderSide enum
        let side = match order.side.to_uppercase().as_str() {
            "BUY" => crate::executor::binance::OrderSide::Buy,
            "SELL" => crate::executor::binance::OrderSide::Sell,
            _ => continue, // Skip invalid side
        };

        // Convert string to OrderType enum
        let order_type = crate::executor::binance::OrderType::Market;

        let quantity = Some(order.amount.clone())
            .and_then(|q| rust_decimal::Decimal::from_str_exact(q.as_str()).ok())
            .map(|q| q.round_dp_with_strategy(3, rust_decimal::RoundingStrategy::ToZero));

        let time_in_force = Some(crate::executor::binance::TimeInForce::Gtc);
        let close_position = None;
        let price = None;

        orders.push(PlaceOrder {
            symbol,
            side,
            position_side: None,
            order_type,
            reduce_only: None,
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
