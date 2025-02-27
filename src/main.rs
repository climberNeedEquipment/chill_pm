use crate::price::fetch_binance_prices;
use anyhow::Result;
use clap::Parser;
use constants::Interval;
use dotenv::dotenv;
use pricedata::BinanceData;
use serde_json::json;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod constants;
pub mod executor;
pub mod feed;
pub mod portfolio;
pub mod price;
pub mod pricedata;
pub mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    // dotenv()?;
    // let args = cli::Args::parse();
    // let onchain_portfolio = get_token_exposure_onchain(&args.wallet_address).await?;

    // let binance_base_url = if args.environment == "test" {
    //     "https://testnet.binancefuture.com"
    // } else {
    //     "https://fapi.binance.com"
    // };
    // let reqwest_cli = reqwest::Client::new();

    // let local = tokio::task::LocalSet::new();

    // let feed_rate = std::time::Duration::from_millis(200);
    // let binance_feed = Arc::new(Mutex::new(
    //     BinanceData::new(&reqwest_cli, 300, &args.symbol).await?,
    // )); // 300 is the window size for support resistance, and obv strategy
    // {
    //     let binance_feed = binance_feed.clone();
    //     let symbol = args.symbol.clone();
    //     local.spawn_local(async move {
    //         loop {
    //             tokio::time::sleep(feed_rate).await;
    //             let price = fetch_binance_prices(&reqwest_cli, &symbol)
    //                 .await
    //                 .expect("Failed to fetch price data");
    //             binance_feed
    //                 .lock()
    //                 .await
    //                 .feed_binance_prices(&reqwest_cli, price)
    //                 .await
    //                 .expect("Failed to feed Binance prices");
    //         }
    //     });
    // }

    // let binance_feed = binance_feed.clone();
    // local.spawn_local(async move {
    //     let user = user::User::new(&browser_client, rand::thread_rng(), (500, 1000));
    //     let mut multi_executor = multi_executor::MultiExecutor::new(
    //         &strategies,
    //         &user,
    //         args.fund,
    //         args.fund / 10.0,
    //         binance_feed,
    //         flipster_feed,
    //     );
    //     multi_executor
    //         .run(args.dry_run)
    //         .await
    //         .expect("Failed to run multi executor");
    // });

    // local.await;
    Ok(())
}
