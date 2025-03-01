use crate::{constants::Interval, utils::price::PriceData};
use anyhow::Result;
use reqwest::Client as ReqwestClient;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use strum::IntoEnumIterator;

#[derive(serde::Serialize, Debug, Clone)]
pub struct OHLCV {
    pub timestamp: u128,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Indicators {
    pub macd: Option<f64>,
    pub rsi: Option<f64>,
}
impl Indicators {
    pub fn all_some(&self) -> bool {
        self.macd.is_some() && self.rsi.is_some()
    }
}

impl Indicators {
    pub fn new() -> Self {
        Self {
            macd: None,
            rsi: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimeframeData {
    pub ohlcv_data: VecDeque<OHLCV>,
    pub ema_short: Option<f64>,
    pub ema_long: Option<f64>,
    pub prev_cur_indicators: (Indicators, Indicators),
}

impl TimeframeData {
    pub fn new(window_size: usize) -> Self {
        Self {
            ohlcv_data: VecDeque::with_capacity(window_size),
            ema_short: None,
            ema_long: None,
            prev_cur_indicators: (Indicators::new(), Indicators::new()),
        }
    }
    pub fn update_indicators(&mut self) -> (Indicators, Indicators) {
        if self.prev_cur_indicators.1.all_some() {
            self.prev_cur_indicators.0 = self.prev_cur_indicators.1.clone();
        }
        self.update_macd();
        self.calculate_rsi(14); // Use a 14-period RSI
        self.prev_cur_indicators.clone()
    }

    pub fn with_initial_data(window_size: usize, initial_data: Vec<OHLCV>) -> Self {
        let mut ohlcv_data = VecDeque::with_capacity(window_size);
        for ohlcv in initial_data {
            ohlcv_data.push_back(ohlcv);
        }
        Self {
            ohlcv_data,
            ema_short: None,
            ema_long: None,
            prev_cur_indicators: (Indicators::new(), Indicators::new()),
        }
    }

    pub fn update_ohlcv(&mut self, new_data: OHLCV) {
        if self.ohlcv_data.back().unwrap().timestamp == new_data.timestamp {
            self.ohlcv_data.pop_back();
        }
        self.ohlcv_data.push_back(new_data);
        if self.ohlcv_data.len() > self.ohlcv_data.capacity() {
            self.ohlcv_data.pop_front();
        }
    }

    fn calculate_ema(&self, current_price: f64, previous_ema: Option<f64>, period: usize) -> f64 {
        let k = 2.0 / (period as f64 + 1.0);
        match previous_ema {
            Some(ema) => (current_price - ema) * k + ema,
            None => current_price,
        }
    }

    fn update_ema(&mut self) {
        let close_price = self.ohlcv_data.back().unwrap().close;

        // Calculate short and long EMAs
        self.ema_short = Some(self.calculate_ema(close_price, self.ema_short, 12));
        self.ema_long = Some(self.calculate_ema(close_price, self.ema_long, 26));
    }

    fn update_macd(&mut self) {
        self.update_ema();
        if let (Some(ema_short), Some(ema_long)) = (self.ema_short, self.ema_long) {
            self.prev_cur_indicators.1.macd = Some(ema_short - ema_long);
        }
    }

    fn calculate_rsi(&mut self, period: usize) {
        if self.ohlcv_data.len() < period + 1 {
            self.prev_cur_indicators.1.rsi = None;
            return;
        }

        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in (self.ohlcv_data.len() - period)..(self.ohlcv_data.len() - 1) {
            let change = self.ohlcv_data[i + 1].close - self.ohlcv_data[i].close;
            if change > 0.0 {
                gains += change;
            } else {
                losses -= change; // Losses are positive
            }
        }

        if gains + losses == 0.0 {
            self.prev_cur_indicators.1.rsi = Some(50.0);
            return;
        }

        let rs = gains / losses;
        self.prev_cur_indicators.1.rsi = Some(100.0 - (100.0 / (1.0 + rs)));
    }
}

pub async fn fetch_binance_ohlcv(
    client: &ReqwestClient,
    timeframe: String,
    limit: usize,
    symbol: &String,
) -> Result<Vec<OHLCV>> {
    // Build the request URL
    let url = "https://fapi.binance.com/fapi/v1/klines";

    // Define the query parameters
    let params = [
        ("symbol", symbol.as_str()),
        ("interval", &timeframe),
        ("limit", &limit.to_string()), // Fetch 'limit' number of candlesticks
    ];

    // Send the GET request
    let response = client
        .get(url)
        .query(&params)
        .send()
        .await?
        .json::<Vec<Vec<Value>>>()
        .await?;

    // Parse the candlestick data
    let mut ohlcv_list = Vec::with_capacity(response.len());
    for kline in response {
        let ohlcv = OHLCV {
            timestamp: kline
                .get(0)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow::anyhow!("Failed to parse timestamp"))?
                as u128,
            open: kline
                .get(1)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to parse open price"))?
                .parse::<f64>()?,
            high: kline
                .get(2)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to parse high price"))?
                .parse::<f64>()?,
            low: kline
                .get(3)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to parse low price"))?
                .parse::<f64>()?,
            close: kline
                .get(4)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to parse close price"))?
                .parse::<f64>()?,
            volume: kline
                .get(5)
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to parse volume"))?
                .parse::<f64>()?,
        };
        ohlcv_list.push(ohlcv);
    }

    Ok(ohlcv_list)
}

pub struct BinanceData {
    pub window_size: usize,
    pub symbol: String,
    pub data: HashMap<Interval, TimeframeData>,
    pub binance_prices: VecDeque<PriceData>,
}

impl BinanceData {
    pub async fn new(client: &ReqwestClient, window_size: usize, symbol: &String) -> Result<Self> {
        let mut data = HashMap::new();
        let futures_fetch_binance_data: Vec<_> = Interval::iter()
            .map(|interval| fetch_binance_ohlcv(client, interval.to_string(), window_size, symbol))
            .collect();

        let ohlcv_data_array: Vec<Result<Vec<OHLCV>>> =
            futures::future::join_all(futures_fetch_binance_data).await;

        for (interval, ohlcv_data) in Interval::iter().zip(ohlcv_data_array) {
            let mut binance_data = TimeframeData::with_initial_data(
                window_size,
                ohlcv_data.expect(
                    format!("Failed to get Binance data for {}", interval.to_string()).as_str(),
                ),
            );
            binance_data.update_indicators();
            data.insert(interval, binance_data);
        }

        Ok(Self {
            window_size,
            symbol: symbol.to_owned(),
            data,
            binance_prices: VecDeque::with_capacity(window_size),
        })
    }

    pub fn get_indicators(&self, interval: Interval) -> Option<(Indicators, Indicators)> {
        self.data
            .get(&interval)
            .map(|data| data.prev_cur_indicators.clone())
    }

    pub async fn feed_binance_prices(
        &mut self,
        client: &ReqwestClient,
        price: PriceData,
    ) -> Result<Vec<(Indicators, Indicators)>> {
        self.binance_prices.push_back(price);
        if self.binance_prices.len() > self.window_size {
            self.binance_prices.pop_front();
        }

        let indicators = self.update_ohlcv_data(client).await?;

        Ok(indicators)
    }

    pub async fn update_ohlcv_data(
        &mut self,
        client: &ReqwestClient,
    ) -> Result<Vec<(Indicators, Indicators)>> {
        let futures_fetch_binance_data: Vec<_> = Interval::iter()
            .map(|interval| fetch_binance_ohlcv(client, interval.to_string(), 1, &self.symbol))
            .collect();

        let results: Vec<Result<Vec<OHLCV>>> =
            futures::future::join_all(futures_fetch_binance_data).await;

        let data_intervals: Vec<OHLCV> = results
            .into_iter()
            .map(|result| {
                result
                    .expect("Failed to get Binance data")
                    .pop()
                    .expect("Empty binance data")
            })
            .collect();

        let mut indicators: Vec<(Indicators, Indicators)> =
            Vec::with_capacity(Interval::iter().count());

        for (interval, ohlcv_data) in Interval::iter().zip(data_intervals) {
            let binance_data = self.data.get_mut(&interval).unwrap();
            binance_data.update_ohlcv(ohlcv_data);
            indicators.push(binance_data.update_indicators());
        }

        Ok(indicators)
    }
}

fn round_to_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[derive(Debug)]
struct BinancePriceData {
    timestamp: u128,
    market_price: f64,
    best_bid: f64,
    best_ask: f64,
}
