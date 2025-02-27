// use crate::constants::WARM_UP_STEPS;
// use crate::executor::Executor;
// use crate::strategy;
// use crate::strategy::StrategyEnum;
// use crate::user;
// use crate::BinanceData;
// use anyhow::Result;
// use std::collections::HashMap;
// use std::sync::Arc;
// use tokio::sync::Mutex;

// pub struct MultiExecutor<'a> {
//     strategies: &'a HashMap<String, Box<StrategyEnum>>,
//     executors: HashMap<String, Executor<'a>>,
//     user: &'a user::User<'a>,
//     fund: f64,
//     stop_fund: f64,
//     total_pnl: f64,
//     total_volume: f64,
//     binance_feed: Arc<Mutex<BinanceData>>,
//     flipster_feed: Arc<Mutex<FlipsterData>>,
// }

// impl<'a> MultiExecutor<'a> {
//     pub fn new(
//         strategies: &'a HashMap<String, Box<StrategyEnum>>,
//         user: &'a user::User<'a>,
//         fund: f64,
//         stop_fund: f64,
//         binance_feed: Arc<Mutex<BinanceData>>,
//     ) -> Self {
//         let mut executors = HashMap::new();
//         for (name, strategy) in strategies {
//             executors.insert(
//                 name.clone(),
//                 Executor::new(
//                     name,
//                     strategy.clone(),
//                     user,
//                     fund,
//                     stop_fund,
//                     binance_feed.clone(),
//                 ),
//             );
//         }

//         Self {
//             strategies,
//             executors,
//             user,
//             fund,
//             stop_fund,
//             total_pnl: 0.0,
//             total_volume: 0.0,
//             binance_feed,
//         }
//     }

//     pub async fn run(&mut self, dry_run: bool) -> Result<()> {
//         let feed_rate = std::time::Duration::from_millis(200);
//         let mut sleep_until = std::time::SystemTime::now();
//         let mut warmup_steps = 0;

//         while self.fund > self.stop_fund {
//             tokio::time::sleep(feed_rate).await;
//             if sleep_until > std::time::SystemTime::now() {
//                 tokio::time::sleep(
//                     sleep_until
//                         .duration_since(std::time::SystemTime::now())
//                         .unwrap(),
//                 )
//                 .await;
//             }

//             let binance_prices = self.binance_feed.lock().await.binance_prices.clone();
//             let binance_data_map = self.binance_feed.lock().await.data.clone();

//             if binance_prices.is_empty() {
//                 println!("Price data not found. Skipping iteration");
//                 tokio::time::sleep(feed_rate).await;
//                 continue;
//             }

//             let binance_price = binance_prices.back().unwrap();

//             let mut pnls: HashMap<String, f64> = HashMap::new();

//             for (name, executor) in &mut self.executors {
//                 let action = executor
//                     .step(&binance_data_map, &binance_price, true)
//                     .await?;
//                 println!("Strategy: {:?} Action: {:?}", name, action);
//                 pnls.insert(name.clone(), executor.get_current_pnl());
//             }

//             if warmup_steps < WARM_UP_STEPS {
//                 warmup_steps += 1;
//                 continue;
//             }

//             let best_strat_name = pnls
//                 .iter()
//                 .max_by(|&(_, value1), &(_, value2)| {
//                     value1
//                         .partial_cmp(value2)
//                         .unwrap_or(std::cmp::Ordering::Equal)
//                 })
//                 .map(|(key, _)| key);

//             let best_executor = self.executors.get_mut(best_strat_name.unwrap()).unwrap();
//             let action = best_executor
//                 .step(&binance_data_map, &binance_price, dry_run)
//                 .await?;

//             match action {
//                 strategy::Action::Hold => {}
//                 _ => {
//                     sleep_until = std::time::SystemTime::now() + std::time::Duration::from_secs(10);
//                 }
//             }
//         }

//         Ok(())
//     }
// }
