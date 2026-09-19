//! Reading the wallet's ORGANIC and Mycel positions: balance from a Solana
//! RPC, price from Jupiter.
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

/// One mint's holding plus the price used to value it.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenPosition {
    pub mint: &'static str,
    pub ticker: &'static str,
    pub balance: f64,
    pub price: f64,
}

impl TokenPosition {
    pub fn usd(&self) -> f64 {
        self.balance * self.price
    }
}

/// Either token at/above the floor wins. If every mint was readable and all
/// are below, the higher USD is returned so the UI can show a real number.
/// A partial failure with nobody clearing the floor is an error — same rule
/// as a bad RPC: do not pretend the wallet holds nothing.
pub fn pick_qualifying_position(
    results: &[Result<TokenPosition, String>],
    floor_usd: f64,
) -> Result<TokenPosition, String> {
    let mut best_ok: Option<&TokenPosition> = None;
    let mut best_any: Option<&TokenPosition> = None;
    let mut first_err: Option<&str> = None;
    let mut n_ok = 0usize;
    for result in results {
        match result {
            Ok(position) => {
                n_ok += 1;
                if best_any.map(|best| position.usd() > best.usd()).unwrap_or(true) {
                    best_any = Some(position);
                }
                if position.usd() >= floor_usd
                    && best_ok.map(|best| position.usd() > best.usd()).unwrap_or(true)
                {
                    best_ok = Some(position);
                }
            }
            Err(err) => {
                if first_err.is_none() {
                    first_err = Some(err);
                }
            }
        }
    }
    if let Some(position) = best_ok {
        return Ok(position.clone());
    }
    if n_ok == results.len() {
        return best_any
            .cloned()
            .ok_or_else(|| "no token positions".to_string());
    }
    Err(first_err.unwrap_or("token position read failed").to_string())
}

async fn read_named(owner: &str, token: &super::GateToken) -> Result<TokenPosition, String> {
    let (balance, price) = read_position(owner, token.mint)
        .await
        .map_err(|e| e.to_string())?;
    Ok(TokenPosition {
        mint: token.mint,
        ticker: token.ticker,
        balance,
        price,
    })
}

/// Best ORGANIC-or-Mycel holding for `owner`.
pub async fn read_qualifying_position(owner: &str) -> Result<TokenPosition, String> {
    let organic = read_named(owner, &super::ORGANIC);
    let mycel = read_named(owner, &super::MYCEL);
    let (organic, mycel) = tokio::join!(organic, mycel);
    pick_qualifying_position(&[organic, mycel], super::FLOOR_USD)
}

/// Total tokens of `mint` held by `owner`, as a UI amount.
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

    fn pos(ticker: &'static str, balance: f64, price: f64) -> TokenPosition {
        TokenPosition {
            mint: ticker,
            ticker,
            balance,
            price,
        }
    }

    #[test]
    fn pick_prefers_the_mint_that_clears_the_floor() {
        let organic = Ok(pos("ORG", 1_000.0, 0.002)); // $2
        let mycel = Ok(pos("MYCEL", 200_000.0, 0.0002)); // $40
        let picked = pick_qualifying_position(&[organic, mycel], 20.0).unwrap();
        assert_eq!(picked.ticker, "MYCEL");
    }

    #[test]
    fn pick_uses_organic_when_it_clears_and_mycel_does_not() {
        let organic = Ok(pos("ORG", 20_000.0, 0.002)); // $40
        let mycel = Ok(pos("MYCEL", 1_000.0, 0.0002)); // $0.20
        let picked = pick_qualifying_position(&[organic, mycel], 20.0).unwrap();
        assert_eq!(picked.ticker, "ORG");
    }

    #[test]
    fn pick_takes_the_higher_usd_when_both_clear() {
        let organic = Ok(pos("ORG", 20_000.0, 0.002)); // $40
        let mycel = Ok(pos("MYCEL", 400_000.0, 0.0002)); // $80
        let picked = pick_qualifying_position(&[organic, mycel], 20.0).unwrap();
        assert_eq!(picked.ticker, "MYCEL");
        assert!((picked.usd() - 80.0).abs() < 1e-9);
    }

    #[test]
    fn pick_returns_the_higher_usd_when_both_are_below() {
        let organic = Ok(pos("ORG", 1_000.0, 0.002)); // $2
        let mycel = Ok(pos("MYCEL", 10_000.0, 0.0002)); // $2
        let picked = pick_qualifying_position(&[organic.clone(), mycel], 20.0).unwrap();
        assert!((picked.usd() - 2.0).abs() < 1e-9);
        let mycel_higher = Ok(pos("MYCEL", 50_000.0, 0.0002)); // $10
        let picked = pick_qualifying_position(&[organic, mycel_higher], 20.0).unwrap();
        assert_eq!(picked.ticker, "MYCEL");
    }

    #[test]
    fn pick_accepts_a_qualifying_mint_even_if_the_other_read_failed() {
        let organic = Err("rpc down".into());
        let mycel = Ok(pos("MYCEL", 200_000.0, 0.0002)); // $40
        let picked = pick_qualifying_position(&[organic, mycel], 20.0).unwrap();
        assert_eq!(picked.ticker, "MYCEL");
    }

    #[test]
    fn pick_errors_on_partial_failure_when_nobody_clears_the_floor() {
        let organic = Err("rpc down".into());
        let mycel = Ok(pos("MYCEL", 1_000.0, 0.0002)); // $0.20
        assert!(pick_qualifying_position(&[organic, mycel], 20.0).is_err());
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
