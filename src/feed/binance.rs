use super::{Feed, Processor};
use crate::{constants::Interval, price::PriceData};
use async_trait::async_trait;
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    error::Error,
};
use strum::IntoEnumIterator;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarketIndexResponse {
    mark_price: String,             // mark price
    index_price: String,            // index price
    estimated_settle_price: String, // Estimated Settle Price, only useful in the last hour before the settlement starts.
    last_funding_rate: String,      // This is the Latest funding rate
    next_funding_time: u64,
    interest_rate: String,
    time: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DepthResponse {
    last_update_id: u64,
    bids: Vec<(String, String)>,
    asks: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FundingRateResponse {
    sym: u64,
    bids: Vec<(String, String)>,
    asks: Vec<(String, String)>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FundingRate {
    symbol: String,
    funding_rate: String,
    funding_time: u64,
    mark_price: String,
}

#[derive(Serialize, Deserialize)]
struct KlineData(
    u64,    // Open time
    String, // Open price
    String, // High price
    String, // Low price
    String, // Close price
    String, // Volume
    u64,    // Close time
    String, // base asset volume
    u64,    // Number of trades
    String, // Taker buy volume
    String, // Taker buy base asset volume
    String, // Ignore
);

#[derive(serde::Serialize, Debug, Clone)]
pub struct OHLCV {
    pub timestamp: u128,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl From<BinanceIndicators> for OHLCV {
    fn from(indicators: BinanceIndicators) -> Self {
        OHLCV {
            timestamp: indicators.ohlcv.timestamp,
            open: indicators.ohlcv.open,
            high: indicators.ohlcv.high,
            low: indicators.ohlcv.low,
            close: indicators.ohlcv.close,
            volume: indicators.ohlcv.volume,
        }
    }
}

struct BinancePriceFeed {
    base_url: String,
    client: ReqwestClient,
    symbol: String,
}

struct BinanceOHLCVFeed {
    base_url: String,
    client: ReqwestClient,
    symbol: String,
    window_size: usize,
}

impl BinancePriceFeed {
    fn new(base_url: String, client: ReqwestClient, symbol: String) -> Self {
        Self {
            base_url,
            client,
            symbol,
        }
    }

    async fn fetch_index_price(&self) -> Result<MarketIndexResponse, reqwest::Error> {
        self.client
            .get(format!("{}/fapi/v1/premiumIndex", self.base_url))
            .query(&[("symbol", self.symbol.as_str())])
            .send()
            .await
            .expect("Failed to send request")
            .json::<MarketIndexResponse>()
            .await
    }

    async fn fetch_market_depth(&self) -> Result<DepthResponse, reqwest::Error> {
        self.client
            .get(format!("{}/fapi/v1/depth", self.base_url))
            .query(&[("symbol", self.symbol.as_str()), ("limit", "5")])
            .send()
            .await
            .expect("Failed to send request")
            .json::<DepthResponse>()
            .await
    }

    async fn fetch_funding_rate(
        &self,
        start_time: u64, // time in ms inclusive
        end_time: u64,
    ) -> Result<FundingRateResponse, reqwest::Error> {
        self.client
            .get(format!("{}/fapi/v1/fundingRate", self.base_url))
            .query(&[
                ("symbol", self.symbol.as_str()),
                ("startTime", &start_time.to_string()),
                ("endTime", &end_time.to_string()),
            ])
            .send()
            .await
            .expect("Failed to send request")
            .json::<FundingRateResponse>()
            .await
    }
}

impl BinanceOHLCVFeed {
    fn new(base_url: String, client: ReqwestClient, symbol: String, window_size: usize) -> Self {
        Self {
            base_url,
            client,
            symbol,
            window_size,
        }
    }

    async fn fetch_binance_ohlcv(
        &self,
        timeframe: String,
    ) -> Result<Vec<OHLCV>, Box<dyn Error + Send + Sync>> {
        // Build the request URL
        let url = format!("{}/fapi/v1/klines", self.base_url);

        // Define the query parameters
        let params = [
            ("symbol", self.symbol.as_str()),
            ("interval", timeframe.as_str()),
            ("limit", &self.window_size.to_string()), // Fetch 'limit' number of candlesticks
        ];

        // Send the GET request
        let response = self
            .client
            .get(url)
            .query(&params)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<KlineData>>()
            .await
            .expect("Failed to parse response");

        // Parse the candlestick data
        let ohlcv_list = response
            .into_iter()
            .map(|kline| {
                Ok(OHLCV {
                    timestamp: kline.0 as u128,
                    open: kline.1.parse::<f64>().expect("Failed to parse open price"),
                    high: kline.2.parse::<f64>().expect("Failed to parse high price"),
                    low: kline.3.parse::<f64>().expect("Failed to parse low price"),
                    close: kline.4.parse::<f64>().expect("Failed to parse close price"),
                    volume: kline.5.parse::<f64>().expect("Failed to parse volume"),
                })
            })
            .collect::<Result<Vec<OHLCV>, Box<dyn Error>>>()
            .expect("Failed to parse OHLCV data");
        Ok(ohlcv_list)
    }
}

#[async_trait]
impl Feed<PriceData> for BinancePriceFeed {
    async fn feed(&self) -> Result<PriceData, Box<dyn Error + Send + Sync>> {
        let (market_index_result, market_depth_result) =
            tokio::join!(self.fetch_index_price(), self.fetch_market_depth());
        let market_index = market_index_result?;
        let market_depth = market_depth_result?;

        Ok(PriceData {
            timestamp: market_index.time.into(),
            market_price: market_index.mark_price.parse::<f64>().ok(),
            buy_long_price: market_depth
                .asks
                .first()
                .map(|x| x.0.parse::<f64>().ok())
                .flatten(),
            sell_short_price: market_depth
                .bids
                .first()
                .map(|x| x.0.parse::<f64>().ok())
                .flatten(),
        })
    }
}

#[async_trait]
impl Feed<HashMap<Interval, Vec<OHLCV>>> for BinanceOHLCVFeed {
    async fn feed(&self) -> Result<HashMap<Interval, Vec<OHLCV>>, Box<dyn Error + Send + Sync>> {
        let mut data = HashMap::new();

        let futures_fetch_binance_data: Vec<_> = Interval::iter()
            .map(|interval| self.fetch_binance_ohlcv(interval.to_string()))
            .collect();

        let ohlcv_data_array: Vec<Result<Vec<OHLCV>, Box<dyn Error + Send + Sync>>> =
            futures::future::join_all(futures_fetch_binance_data).await;

        for (interval, ohlcv_data) in Interval::iter().zip(ohlcv_data_array) {
            let ohlcv = ohlcv_data?;
            data.insert(interval, ohlcv);
        }
        Ok(data)
    }
}

struct BinanceIndicators {
    ohlcv: OHLCV,
    rsi: f64,
    ema_short: f64,
    ema_long: f64,
}

struct BinanceOHLCVProcessor {
    data: VecDeque<OHLCV>,
    size: usize,
    rsi_period: usize,
    ema_short_period: usize,
    ema_long_period: usize,
    ema_short: Option<f64>,
    ema_long: Option<f64>,
}

impl BinanceOHLCVProcessor {
    fn calculate_ema(&self, current_price: f64, previous_ema: Option<f64>, period: usize) -> f64 {
        let k = 2.0 / (period as f64 + 1.0);
        match previous_ema {
            Some(ema) => (current_price - ema) * k + ema,
            None => self.data.iter().map(|p| p.close).sum::<f64>() / self.data.len() as f64,
        }
    }

    fn calculate_rsi(&self) -> Option<f64> {
        if self.data.len() < self.rsi_period + 1 {
            return None;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in (self.data.len() - self.rsi_period)..(self.data.len() - 1) {
            let change = self.data[i + 1].close - self.data[i].close;
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change; // losses are positive
            }
        }

        if gains + losses == 0.0 {
            return Some(50.0);
        }

        let rs = gains / losses;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }
}

#[async_trait]
impl Processor<BinanceIndicators, OHLCV> for BinanceOHLCVProcessor {
    async fn process(
        &self,
        data: &OHLCV,
    ) -> Result<BinanceIndicators, Box<dyn Error + Send + Sync>> {
        let rsi = self.calculate_rsi();
        let ema_short = self.calculate_ema(data.close, self.ema_short, self.ema_short_period);
        let ema_long = self.calculate_ema(data.close, self.ema_long, self.ema_long_period);

        Ok(BinanceIndicators {
            ohlcv: data.to_owned(),
            rsi: rsi.unwrap_or(0.0),
            ema_short,
            ema_long,
        })
    }
}
