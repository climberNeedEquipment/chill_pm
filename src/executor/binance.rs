use crate::executor::error::ExchangeError;
use crate::utils::sign::BinanceKey;
use anyhow::Result;
use positions::Asset;
use reqwest::header::HeaderValue;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Position side.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PositionSide {
    /// Long.
    Long,
    /// Short.
    Short,
    /// Both.
    Both,
}

/// Order types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderKind {
    /// Market.
    Market,
    /// Limit.
    Limit(Decimal, TimeInForceKind),
    /// Post-Only.
    PostOnly(Decimal),
}

/// Time in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInForceKind {
    /// Good-Til-Cancelled.
    GoodTilCancelled,
    /// Fill-Or-Kill.
    FillOrKill,
    /// Immediate-Or-Cancel.
    ImmediateOrCancel,
}

/// Order Status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderStatus {
    /// Pending.
    Pending,
    /// Finished.
    Finished,
    /// Unknown.
    Unknown,
}

/// Order State.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderState {
    /// Filled size.
    pub filled: Decimal,
    /// Average cost.
    pub cost: Decimal,
    /// Status.
    pub status: OrderStatus,
    /// Fees.
    pub fees: HashMap<Asset, Decimal>,
}

/// Status.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    /// New.
    #[serde(alias = "ACCEPTED")]
    New,
    /// Parttially filled.
    PartiallyFilled,
    /// Filled.
    Filled,
    /// Cancelled.
    #[serde(alias = "REJECTED", alias = "CANCELLED")]
    Canceled,
    /// Expired.
    Expired,
    /// New insurance.
    NewInsurance,
    /// New ADL.
    NewAdl,
}

impl TryFrom<Status> for OrderStatus {
    type Error = ExchangeError;

    fn try_from(status: Status) -> Result<Self, Self::Error> {
        let status = match status {
            Status::New | Status::PartiallyFilled => OrderStatus::Pending,
            Status::Canceled | Status::Expired | Status::Filled => OrderStatus::Finished,
            Status::NewAdl | Status::NewInsurance => OrderStatus::Pending,
        };
        Ok(status)
    }
}

impl Default for OrderState {
    fn default() -> Self {
        Self {
            filled: Decimal::ZERO,
            cost: Decimal::ONE,
            status: OrderStatus::Pending,
            fees: HashMap::default(),
        }
    }
}

/// Order trade.
#[derive(Debug, Clone)]
pub struct OrderTrade {
    /// Price.
    pub price: Decimal,
    /// Size.
    pub size: Decimal,
    /// Fee.
    pub fee: Decimal,
    /// Fee asset.
    pub fee_asset: Option<Asset>,
}
/// Order type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    /// Market.
    Market,
    /// Limit.
    Limit,
    /// Stop.
    Stop,
    /// Take-Profit.
    TakeProfit,
    /// Stop-Market.
    StopMarket,
    /// Take-Profit-Market.
    TakeProfitMarket,
    /// Trailing-Stop-Market.
    TrailingStopMarket,
    /// Limit Maker.
    LimitMaker,
}

/// Order side.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    /// Buy.
    Buy,
    /// Sell.
    Sell,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceOrder {
    /// Symbol.
    pub symbol: String,
    /// Side.
    pub side: OrderSide,
    /// Position side.
    pub position_side: Option<PositionSide>,
    /// Order type.
    #[serde(rename = "type")]
    pub order_type: OrderType,
    /// Reduce only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<bool>,
    /// Quantity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Decimal>,
    /// Price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<Decimal>,
    /// Client id.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_client_order_id: Option<String>,
    /// Stop price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<Decimal>,
    /// Close position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_position: Option<bool>,
    /// Activation price.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<Decimal>,
    /// Callback rate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<Decimal>,
    /// Time-In-Force.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    /// Working type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<String>,
    /// Price protect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<String>,
}

/// Usd-Margin Futures Order.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdMarginFuturesOrder {
    /// Client id.
    pub client_order_id: String,
    /// FIXME: what is this?
    pub cum_qty: Option<Decimal>,
    /// FIXME: what is this?
    pub cum_quote: Option<Decimal>,
    /// Filled size.
    pub executed_qty: Decimal,
    /// Order id.
    pub order_id: i64,
    /// Cost.
    pub avg_price: Decimal,
    /// Size.
    pub orig_qty: Decimal,
    /// Price.
    pub price: Decimal,
    /// Reduce only.
    pub reduce_only: bool,
    /// Order side.
    pub side: OrderSide,
    /// Position side.
    pub position_side: PositionSide,
    /// Status.
    pub status: Status,
    /// Stop price.
    pub stop_price: Decimal,
    /// Is close position.
    pub close_position: bool,
    /// Symbol.
    pub symbol: String,
    /// Time-In-Force.
    pub time_in_force: TimeInForce,
    /// Order type.
    #[serde(rename = "type")]
    pub order_type: OrderType,
    /// Active price.
    pub activate_price: Option<Decimal>,
    /// Price rate.
    pub price_rate: Option<Decimal>,
    /// Update timestamp.
    pub update_time: i64,
    /// Working type.
    pub working_type: String,
    /// Price protect.
    pub price_protect: bool,
}

/// Time-in-force.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TimeInForce {
    /// Good-Till-Cancel.
    Gtc,
    /// Immdiate-Or-Cancel.
    Ioc,
    /// Fill-Or-Kill.
    Fok,
    /// Post-Only.
    Gtx,
}

pub async fn place_binance_order(
    base_url: &str,
    key: &BinanceKey,
    symbol: &str,
    side: OrderSide,
    quantity: Option<Decimal>,
    price: Option<Decimal>,
    stop_price: Option<Decimal>,
) -> Result<UsdMarginFuturesOrder> {
    // Create an empty parameter map to sign
    let (order_type, time_in_force, close_position) = if price.is_some() {
        (OrderType::Limit, Some(TimeInForce::Gtc), Some(false))
    } else if stop_price.is_some() {
        (OrderType::StopMarket, None, Some(true))
    } else {
        (OrderType::Market, None, Some(false))
    };

    let place_order_params = PlaceOrder {
        symbol: format!("{}{}", symbol.to_uppercase(), "USDT"),
        side,
        position_side: None,
        order_type,
        reduce_only: None,
        quantity: quantity,
        price,
        new_client_order_id: None,
        stop_price,
        close_position,
        activation_price: None,
        callback_rate: None,
        time_in_force,
        working_type: None,
        price_protect: None,
    };

    // Sign the parameters
    let signed_params = key
        .sign(place_order_params)
        .map_err(|e| anyhow::anyhow!("Error signing parameters: {}", e))?;

    // Construct the full URL with the signed query string
    let url = format!("{}/fapi/v1/order", base_url);
    let hyper_body = serde_urlencoded::to_string(signed_params)?;
    // Create a client and set the necessary headers
    let client = Client::new();
    let response = client
        .post(&url)
        .header(
            "X-MBX-APIKEY",
            HeaderValue::from_str(&key.api_key)
                .map_err(|e| anyhow::anyhow!("Invalid API key: {}", e))?,
        )
        .body(hyper_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to get order status: HTTP {}",
            response.status()
        ));
    }

    let order: UsdMarginFuturesOrder = response.json().await?;
    Ok(order)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_place_binance_order() -> Result<()> {
    dotenv().unwrap();
    let binance_key = BinanceKey {
        api_key: env::var("BINANCE_API_KEY").expect("BINANCE_API_KEY must be set in .env"),
        secret_key: env::var("BINANCE_API_SECRET").expect("BINANCE_SECRET_KEY must be set in .env"),
    };
    let binance_base_url =
        if env::var("ENVIRONMENT").expect("BINANCE_ENV must be set in .env") == "test" {
            "https://testnet.binancefuture.com"
        } else {
            "https://fapi.binance.com"
        };

    // market order
    // let order = place_binance_order(
    //     &binance_base_url,
    //     &binance_key,
    //     "ETH",
    //     OrderSide::Buy,
    //     Some(Decimal::from(1)),
    //     Some(Decimal::from_i128_with_scale(260812i128, 2)),
    //     None,
    // )
    // .await?;

    // market close
    let order = place_binance_order(
        &binance_base_url,
        &binance_key,
        "ETH",
        OrderSide::Sell,
        None,
        None,
        Some(Decimal::from_i128_with_scale(262312i128, 2)),
    )
    .await?;
    println!("{:?}", order);
    Ok(())
}
