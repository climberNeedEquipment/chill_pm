// https://api.eigenexplorer.com/stakers/{address}

// etherfi
// https://api.sevenseas.capital/info
// ['ethereum', 'arbitrum', 'avalanche', 'base', 'corn', 'bnb', 'sonic', 'swell', 'bob'
// https://api.sevenseas.capital/etherfi/apy/<network>/<vault_address>
//

use reqwest::Error;
use serde::Deserialize;

struct Protocols {
    data: HashMap<String, f64>,
}

#[derive(Deserialize, Debug)]
struct VaultData {
    address: String,
    block: u64,
    timestamp: String,
    apy: Protocols,
    #[serde(rename = "7_day_apy")]
    seven_day_apy: Protocols,
    #[serde(rename = "14_day_apy")]
    fourteen_day_apy: Protocols,
    #[serde(rename = "30_day_apy")]
    thirty_day_apy: Protocols,
    allocation: Protocols,
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    response: Vec<VaultData>,
}

// kelp dao
// https://universe.kelpdao.xyz/rseth/totalApy
// https://universe.kelpdao.xyz/rseth/gainApy
