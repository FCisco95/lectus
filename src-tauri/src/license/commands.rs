//! Tauri surface for the Organic token gate: link, unlink, status, refresh.

use super::{
    apply_check, chain, connect, license_file, load_from, save_to, verify, License, LicenseState,
    Status, GATE_TOKENS, ORGANIC,
};
use std::path::PathBuf;
use tauri::{Emitter, Manager};

/// Where `license.json` lives (beside `history.json` in the app data dir).
fn license_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| license_file(&d))
}

/// Read the stored link at startup, before any window exists.
pub fn load_at_startup(dir: &std::path::Path) -> Option<License> {
    load_from(&license_file(dir))
}

fn persist(app: &tauri::AppHandle, license: &License) {
    let Some(path) = license_path(app) else { return };
    if let Err(e) = save_to(&path, license) {
        log::warn!("could not save license.json: {e}");
    }
}

/// Tell the UI the gate moved. Dictation state is never touched here — a
/// status change must not interrupt a dictation already in flight.
fn announce(app: &tauri::AppHandle, status: &Status) {
    let _ = app.emit("license-changed", status.clone());
}

/// Current gate verdict for the UI.
#[tauri::command]
pub fn license_status(license_state: tauri::State<LicenseState>) -> Status {
    license_state.status(crate::now_millis())
}

/// Token floor, and what it costs in ORGANIC and Mycel right now.
#[tauri::command]
pub async fn license_floor() -> Result<serde_json::Value, String> {
    let mut tokens = Vec::new();
    for token in GATE_TOKENS {
        match chain::token_price_usd(token.mint).await {
            Ok(price) => tokens.push(serde_json::json!({
                "mint": token.mint,
                "ticker": token.ticker,
                "name": token.name,
                "price_usd": price,
                "tokens_required": super::tokens_required(price),
            })),
            Err(e) => log::warn!("price for {} failed: {e}", token.ticker),
        }
    }
    let organic = tokens.iter().find(|t| t["ticker"] == ORGANIC.ticker);
    Ok(serde_json::json!({
        "floor_usd": super::FLOOR_USD,
        "tokens": tokens,
        "price_usd": organic.and_then(|t| t["price_usd"].as_f64()),
        "tokens_required": organic.and_then(|t| t["tokens_required"].as_f64()),
        "mint": ORGANIC.mint,
    }))
}

/// Open the browser, wait for a wallet signature, then read the chain.
///
/// Long-running by design: it returns only once the user has signed or the
/// attempt has timed out. The UI shows "waiting for your wallet" meanwhile.
#[tauri::command]
pub async fn link_wallet(
    app_handle: tauri::AppHandle,
    license_state: tauri::State<'_, LicenseState>,
) -> Result<Status, String> {
    let nonce = verify::new_nonce();
    *license_state.pending_nonce.lock().unwrap() = Some(nonce.clone());

    let session = connect::start(&nonce).map_err(|e| e.to_string())?;
    connect::open_in_browser(&session.url).map_err(|e| e.to_string())?;
    log::info!("wallet link page opened at {}", session.url);

    // The link server blocks its own thread; keep it off the async runtime.
    let proof = tauri::async_runtime::spawn_blocking(move || session.proof.recv())
        .await
        .map_err(|e| format!("wallet link task failed: {e}"))?
        .map_err(|_| "wallet link was closed before signing".to_string())?
        .map_err(|e| e.to_string())?;

    // The nonce is single-use: consume it, so a replayed POST cannot re-link.
    let expected = license_state.pending_nonce.lock().unwrap().take();
    let expected = expected.ok_or_else(|| "no link attempt in progress".to_string())?;
    verify::verify_signature(&proof.pubkey, &proof.signature, &expected)
        .map_err(|e| e.to_string())?;

    let now = crate::now_millis();
    let mut license = License {
        pubkey: proof.pubkey.clone(),
        linked_at_ms: now,
        ..Default::default()
    };

    // A link with an unreachable chain is still a link: the wallet is proven,
    // and the balance read retries in the background.
    match chain::read_qualifying_position(&proof.pubkey).await {
        Ok(pos) => apply_check(&mut license, pos.balance, pos.price, pos.ticker, now),
        Err(e) => log::warn!("wallet linked but the balance read failed: {e}"),
    }

    persist(&app_handle, &license);
    *license_state.license.lock().unwrap() = Some(license.clone());

    let status = super::status_of(Some(&license), now);
    announce(&app_handle, &status);
    log::info!("wallet linked: {} — {:?}", proof.pubkey, status);
    Ok(status)
}

/// Forget the linked wallet.
#[tauri::command]
pub fn unlink_wallet(
    app_handle: tauri::AppHandle,
    license_state: tauri::State<LicenseState>,
) -> Result<Status, String> {
    *license_state.license.lock().unwrap() = None;
    if let Some(path) = license_path(&app_handle) {
        let _ = std::fs::remove_file(path);
    }
    let status = Status::Unlinked;
    announce(&app_handle, &status);
    Ok(status)
}

/// Re-read the linked wallet's balance now, ignoring the usual cadence.
#[tauri::command]
pub async fn refresh_license(
    app_handle: tauri::AppHandle,
    license_state: tauri::State<'_, LicenseState>,
) -> Result<Status, String> {
    let Some(mut license) = license_state.license.lock().unwrap().clone() else {
        return Ok(Status::Unlinked);
    };

    let pos = chain::read_qualifying_position(&license.pubkey).await?;

    let now = crate::now_millis();
    apply_check(&mut license, pos.balance, pos.price, pos.ticker, now);
    persist(&app_handle, &license);
    *license_state.license.lock().unwrap() = Some(license.clone());

    let status = super::status_of(Some(&license), now);
    announce(&app_handle, &status);
    Ok(status)
}

/// Background re-check on launch: silent, and only when the cadence says so.
///
/// Every failure path is log-only. A node that is down, a laptop offline in a
/// plane — neither may cost the user their dictation; the grace window in
/// `status_of` covers that.
pub fn spawn_background_check(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<LicenseState>();
        let Some(mut license) = state.license.lock().unwrap().clone() else {
            return;
        };
        let now = crate::now_millis();
        if !super::is_check_due(&license, now) {
            return;
        }

        match chain::read_qualifying_position(&license.pubkey).await {
            Ok(pos) => {
                let now = crate::now_millis();
                apply_check(&mut license, pos.balance, pos.price, pos.ticker, now);
                persist(&app, &license);
                *state.license.lock().unwrap() = Some(license.clone());
                let status = super::status_of(Some(&license), now);
                log::info!("license re-checked: {status:?}");
                announce(&app, &status);
            }
            Err(e) => log::warn!("license re-check failed (staying on cached status): {e}"),
        }
    });
}
