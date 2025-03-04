use crate::portfolio;
use crate::portfolio::binance::AccountInfo;

// Helper function to format Binance portfolio data
pub fn format_binance_portfolio(account_info: &AccountInfo) -> String {
    let mut output = String::new();

    output.push_str(&format!("Binance Portfolio Summary:\n"));
    output.push_str(&format!(
        "Wallet Balance: {}\n",
        account_info.total_wallet_balance
    ));
    output.push_str(&format!(
        "Unrealized Profit: {}\n",
        account_info.total_unrealized_profit
    ));
    output.push_str(&format!(
        "Margin Balance: {}\n",
        account_info.total_margin_balance
    ));

    // Format assets
    if !account_info.assets.is_empty() {
        output.push_str("\nAssets:\n");
        for (i, asset) in account_info.assets.iter().enumerate().take(5) {
            output.push_str(&format!(
                "  Asset {}: {} - Balance: {}\n",
                i + 1,
                asset.asset,
                asset.wallet_balance
            ));
        }
        if account_info.assets.len() > 5 {
            output.push_str(&format!(
                "  ... and {} more assets\n",
                account_info.assets.len() - 5
            ));
        }
    }

    // Format positions
    let active_positions: Vec<_> = account_info
        .positions
        .iter()
        .filter(|p| p.position_amt != "0")
        .collect();

    if !active_positions.is_empty() {
        output.push_str("\nActive Positions:\n");
        for (i, position) in active_positions.iter().enumerate().take(5) {
            output.push_str(&format!(
                "  Position {}: {} - Amount: {}, Unrealized PnL: {}\n",
                i + 1,
                position.symbol,
                position.position_amt,
                position.unrealized_profit
            ));
        }
        if active_positions.len() > 5 {
            output.push_str(&format!(
                "  ... and {} more positions\n",
                active_positions.len() - 5
            ));
        }
    }

    output
}

// Helper function to format Eisen onchain data
pub fn format_onchain_data<T>(onchain_data: &T) -> String
where
    T: std::fmt::Display,
{
    format!("Onchain Portfolio Data:\n{}", onchain_data)
}
