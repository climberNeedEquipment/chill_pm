
// Application state that will be shared between handlers
#[derive(Clone)]
pub struct AppState {
    pub binance_base_url: String,
    pub binance_api_key: String,
    pub binance_api_secret: String,
    pub eisen_base_url: String,
    pub reqwest_cli: reqwest::Client,
}
