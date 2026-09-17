//! Reading the wallet's ORGANIC position: balance from a Solana RPC, price
//! from Jupiter.
//!
//! Both are plain public HTTP endpoints. No API key ships in the binary — the
//! repo is public, so an embedded key would leak with every download. Users on
//! a rate-limited public RPC can point `LECTUS_RPC_URL` at their own.

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";
const JUPITER_PRICE_URL: &str = "https://lite-api.jup.ag/price/v3";
const HTTP_TIMEOUT_SECS: u64 = 15;

/// RPC endpoint to read balances from. Override with `LECTUS_RPC_URL` (e.g. a
/// personal Helius URL) when the public node rate-limits.
pub fn rpc_url() -> String {
    std::env::var("LECTUS_RPC_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string())
}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()?)
}

/// Total ORGANIC held by `owner`, as a UI amount.
///
/// A wallet can hold the same mint across several token accounts, so every
/// account is summed rather than taking the first.
pub async fn token_balance(owner: &str, mint: &str) -> Result<f64> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            owner,
            { "mint": mint },
            { "encoding": "jsonParsed", "commitment": "confirmed" }
        ]
    });

    let response: Value = client()?
        .post(rpc_url())
        .json(&body)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    parse_token_balance(&response)
}

/// Sum `uiAmount` across the token accounts in a `getTokenAccountsByOwner`
/// reply. A wallet that has never held the mint returns an empty list, which
/// is a balance of zero, not an error.
pub fn parse_token_balance(response: &Value) -> Result<f64> {
    if let Some(err) = response.get("error") {
        return Err(anyhow!("rpc error: {err}"));
    }
    let accounts = response
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("rpc reply has no result.value array"))?;

    let mut total = 0.0;
    for account in accounts {
        let amount = account
            .pointer("/account/data/parsed/info/tokenAmount/uiAmount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        total += amount;
    }
    Ok(total)
}

/// Current ORGANIC/USD price.
pub async fn token_price_usd(mint: &str) -> Result<f64> {
    let response: Value = client()?
        .get(JUPITER_PRICE_URL)
        .query(&[("ids", mint)])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    parse_price(&response, mint)
}

/// Pull `usdPrice` for one mint out of a Jupiter price reply.
pub fn parse_price(response: &Value, mint: &str) -> Result<f64> {
    let price = response
        .get(mint)
        .and_then(|entry| entry.get("usdPrice"))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| anyhow!("no usdPrice for {mint}"))?;

    // A zero or negative price would make any balance clear a dollar floor.
    if !price.is_finite() || price <= 0.0 {
        return Err(anyhow!("unusable price for {mint}: {price}"));
    }
    Ok(price)
}

/// One full read: balance and price together.
pub async fn read_position(owner: &str, mint: &str) -> Result<(f64, f64)> {
    let (balance, price) = tokio::join!(token_balance(owner, mint), token_price_usd(mint));
    Ok((balance?, price?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn account(ui_amount: f64) -> Value {
        json!({
            "account": { "data": { "parsed": { "info": {
                "tokenAmount": { "uiAmount": ui_amount, "decimals": 6 }
            }}}}
        })
    }

    #[test]
    fn sums_every_token_account_for_the_mint() {
        let reply = json!({ "result": { "value": [account(1000.5), account(2000.25)] } });
        assert!((parse_token_balance(&reply).unwrap() - 3000.75).abs() < 1e-9);
    }

    #[test]
    fn no_token_account_is_a_zero_balance() {
        let reply = json!({ "result": { "value": [] } });
        assert_eq!(parse_token_balance(&reply).unwrap(), 0.0);
    }

    #[test]
    fn rpc_error_is_an_error_not_a_zero_balance() {
        // Must not read as "holds nothing" — that would lock a real holder out
        // on a bad node instead of leaving them in grace.
        let reply = json!({ "error": { "code": -32000, "message": "node behind" } });
        assert!(parse_token_balance(&reply).is_err());
    }

    #[test]
    fn malformed_reply_is_an_error() {
        assert!(parse_token_balance(&json!({ "result": {} })).is_err());
    }

    #[test]
    fn reads_the_jupiter_price() {
        let reply = json!({ "MINT": { "usdPrice": 0.0025220802712995127, "decimals": 6 } });
        assert!((parse_price(&reply, "MINT").unwrap() - 0.00252208).abs() < 1e-8);
    }

    #[test]
    fn rejects_a_missing_or_zero_price() {
        assert!(parse_price(&json!({}), "MINT").is_err());
        assert!(parse_price(&json!({ "MINT": { "usdPrice": 0.0 } }), "MINT").is_err());
    }

    #[test]
    fn rpc_url_defaults_and_honours_the_override() {
        // Default holds when the override is unset or blank.
        std::env::remove_var("LECTUS_RPC_URL");
        assert_eq!(rpc_url(), DEFAULT_RPC_URL);
        std::env::set_var("LECTUS_RPC_URL", "   ");
        assert_eq!(rpc_url(), DEFAULT_RPC_URL);
        std::env::set_var("LECTUS_RPC_URL", "https://example.test/rpc");
        assert_eq!(rpc_url(), "https://example.test/rpc");
        std::env::remove_var("LECTUS_RPC_URL");
    }
}
