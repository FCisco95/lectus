//! Official public-client Sign in with ChatGPT protocol, verified 2026-10-22.
//! https://developers.openai.com/siwc/token-sharing-open-source/sign-in
use super::store::{Registration, Tokens};
use super::*;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};
use tauri_plugin_shell::ShellExt;

const ISSUER: &str = "https://auth.openai.com";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const RESOURCE: &str = "https://api.openai.com/v1";
const SCOPES: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

#[derive(Debug, Deserialize)]
struct Identity {
    sub: String,
    email: Option<String>,
    nonce: Option<String>,
    iat: Option<u64>,
}
fn verify_identity(
    token: &str,
    jwks: &JwkSet,
    client_id: &str,
    nonce: Option<&str>,
    subject: Option<&str>,
) -> Result<Identity> {
    let invalid = || anyhow!("ChatGPT identity validation failed. Start a fresh sign-in.");
    let header = decode_header(token).map_err(|_| invalid())?;
    if !matches!(
        header.alg,
        Algorithm::RS256 | Algorithm::ES256 | Algorithm::EdDSA
    ) {
        return Err(invalid());
    }
    let key = jwks
        .find(header.kid.as_deref().ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    let key = DecodingKey::from_jwk(key).map_err(|_| invalid())?;
    let mut validation = Validation::new(header.alg);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[client_id]);
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub", "iat"]);
    validation.leeway = 30;
    validation.validate_nbf = true;
    let claims = decode::<Identity>(token, &key, &validation)
        .map_err(|_| invalid())?
        .claims;
    if claims.sub.is_empty()
        || nonce.is_some_and(|n| claims.nonce.as_deref() != Some(n))
        || subject.is_some_and(|s| s != claims.sub)
        || claims.iat.is_some_and(|t| t > store::now() + 30)
    {
        return Err(invalid());
    }
    Ok(claims)
}
fn jwks(http: &reqwest::blocking::Client) -> Result<JwkSet> {
    let response = http
        .get(format!("{ISSUER}/.well-known/jwks.json"))
        .send()
        .map_err(|_| anyhow!("Cannot fetch ChatGPT identity verification keys."))?;
    serde_json::from_value(json_response(response)?)
        .map_err(|_| anyhow!("ChatGPT identity verification keys are invalid."))
}
fn has_permission(scope: &str) -> bool {
    ["openid", "resource.invoke", "chatgpt.tokens.use.direct"]
        .iter()
        .all(|required| scope.split_whitespace().any(|s| s == *required))
}
fn parse_tokens(value: &Value, previous: Option<&Tokens>) -> Result<Tokens> {
    if !value["token_type"]
        .as_str()
        .is_some_and(|s| s.eq_ignore_ascii_case("bearer"))
    {
        bail!("ChatGPT returned an unsupported credential type.");
    }
    let access_token = value["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("ChatGPT did not return an access credential."))?
        .to_string();
    let refresh_token = value["refresh_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| previous.map(|t| t.refresh_token.clone()))
        .ok_or_else(|| anyhow!("ChatGPT did not grant renewable access. Sign in again."))?;
    let id_token = value["id_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| previous.map(|t| t.id_token.clone()))
        .ok_or_else(|| anyhow!("ChatGPT did not return a verified identity."))?;
    let scope = value["scope"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| previous.map(|t| t.scope.clone()))
        .ok_or_else(|| anyhow!("ChatGPT did not return granted permissions."))?;
    if !has_permission(&scope) {
        bail!("ChatGPT plan usage was not granted. Sign in and allow ChatGPT plan usage.");
    }
    let expires = value["expires_in"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= 86400 * 30)
        .ok_or_else(|| anyhow!("ChatGPT returned an invalid credential expiry."))?;
    Ok(Tokens {
        access_token,
        refresh_token,
        id_token,
        scope,
        expires_at: store::now() + expires,
    })
}
fn authorization_url(
    host: &str,
    registration: &Registration,
    redirect: &str,
    state: &str,
    nonce: &str,
    verifier: &str,
) -> Result<reqwest::Url> {
    use base64::Engine;
    use sha2::{Digest, Sha256};
    let mut url = reqwest::Url::parse("https://auth.openai.com/api/accounts/authorize")
        .map_err(|_| anyhow!("Cannot prepare ChatGPT sign-in."))?;
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(Sha256::digest(verifier.as_bytes()));
    let mut params = url.query_pairs_mut();
    params
        .append_pair(
            "client_id",
            if registration.client_id.is_empty() {
                "dynamic_agent_client"
            } else {
                &registration.client_id
            },
        )
        .append_pair("ext_agent_host_id", host)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", redirect)
        .append_pair("scope", SCOPES)
        .append_pair("resource", RESOURCE)
        .append_pair("state", state)
        .append_pair("nonce", nonce)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &challenge);
    if registration.client_id.is_empty() {
        params.append_pair("agent_name_hint", "Lectus");
    } else if let Some(tokens) = &registration.tokens {
        params.append_pair("id_token_hint", &tokens.id_token);
    }
    drop(params);
    Ok(url)
}
fn callback(target: &str, state: &str, previous_client: &str) -> Result<(String, String)> {
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| anyhow!("Invalid ChatGPT sign-in callback."))?;
    if url.path() != "/auth/callback" || url.fragment().is_some() {
        bail!("Invalid ChatGPT sign-in callback.");
    }
    let mut query = HashMap::new();
    for (key, value) in url.query_pairs() {
        if query.insert(key.into_owned(), value.into_owned()).is_some() {
            bail!("Invalid duplicate sign-in parameters.");
        }
    }
    if query.get("state").map(String::as_str) != Some(state) {
        bail!("ChatGPT sign-in state did not match.");
    }
    if query.contains_key("error") {
        bail!("ChatGPT sign-in was declined. Start again to allow plan usage.");
    }
    let code = query
        .get("code")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("ChatGPT sign-in did not return a code."))?;
    let client_id = query
        .get("client_id")
        .map(String::as_str)
        .unwrap_or(previous_client);
    if client_id.is_empty()
        || client_id == "dynamic_agent_client"
        || (!previous_client.is_empty() && client_id != previous_client)
    {
        bail!("ChatGPT registration did not match. Start a fresh sign-in.");
    }
    Ok((code.clone(), client_id.into()))
}
fn callback_request(stream: &mut TcpStream, port: u16) -> Result<String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| anyhow!("Cannot read sign-in callback."))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|_| anyhow!("Cannot reply to sign-in callback."))?;
    let mut bytes = Vec::new();
    let mut part = [0; 1024];
    while !bytes.windows(4).any(|s| s == b"\r\n\r\n") {
        let n = stream
            .read(&mut part)
            .map_err(|_| anyhow!("Invalid sign-in callback request."))?;
        if n == 0 || bytes.len() + n > 16 * 1024 {
            bail!("Invalid sign-in callback request.");
        }
        bytes.extend_from_slice(&part[..n]);
    }
    let request =
        std::str::from_utf8(&bytes).map_err(|_| anyhow!("Invalid sign-in callback request."))?;
    let mut lines = request.split("\r\n");
    let first = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>();
    if first.len() != 3 || first[0] != "GET" || !first[1].starts_with("/auth/callback?") {
        bail!("Invalid sign-in callback request.");
    }
    let expected_host = format!("127.0.0.1:{port}");
    if !lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("host") && value.trim() == expected_host
        })
    }) {
        bail!("Invalid sign-in callback host.");
    }
    Ok(first[1].into())
}
pub(super) fn login(lease: Lease, app: &tauri::AppHandle) -> Result<ConnectionStatus> {
    let previous = {
        let _file = store::lock(lease)?;
        let mut vault = store::load()?;
        if vault.host_id.is_empty() {
            vault.host_id = format!("lectus:{}", store::random()?);
            lease.commit(|| store::save(&vault))?;
        }
        (vault.host_id, vault.registration)
    };
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|_| anyhow!("Cannot start ChatGPT loopback sign-in listener."))?;
    listener
        .set_nonblocking(true)
        .map_err(|_| anyhow!("Cannot prepare sign-in listener."))?;
    let port = listener
        .local_addr()
        .map_err(|_| anyhow!("Cannot prepare sign-in callback."))?
        .port();
    let redirect = format!("http://127.0.0.1:{port}/auth/callback");
    let state = store::random()?;
    let nonce = store::random()?;
    let verifier = store::random()?;
    let url = authorization_url(
        &previous.0,
        &previous.1,
        &redirect,
        &state,
        &nonce,
        &verifier,
    )?;
    lease.check()?;
    #[allow(deprecated)]
    app.shell()
        .open(url.as_str(), None)
        .map_err(|_| anyhow!("Cannot open your system browser for ChatGPT sign-in."))?;
    let started = Instant::now();
    let (code, client_id) = loop {
        lease.check()?;
        if started.elapsed() > Duration::from_secs(300) {
            bail!("ChatGPT sign-in timed out. Start again.");
        }
        match listener.accept() {
            Ok((mut stream, peer)) => {
                if !peer.ip().is_loopback() {
                    continue;
                }
                let target = callback_request(&mut stream, port);
                let matches_state = target.as_ref().ok().is_some_and(|target| {
                    reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
                        .ok()
                        .is_some_and(|url| {
                            let values = url
                                .query_pairs()
                                .filter(|(key, _)| key == "state")
                                .map(|(_, value)| value.into_owned())
                                .collect::<Vec<_>>();
                            values == [state.as_str()]
                        })
                });
                let parsed =
                    target.and_then(|target| callback(&target, &state, &previous.1.client_id));
                let (http_status, body) = if parsed.is_ok() {
                    (
                        "200 OK",
                        "Sign-in received. Return to Lectus to check the connection.",
                    )
                } else {
                    (
                        "400 Bad Request",
                        "Sign-in callback rejected. Return to Lectus.",
                    )
                };
                let _ = write!(stream, "HTTP/1.1 {http_status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Security-Policy: default-src 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}", body.len());
                // An unsolicited local request must not consume the pending attempt.
                if matches_state {
                    break parsed?;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(50))
            }
            Err(_) => bail!("ChatGPT sign-in listener failed. Start again."),
        }
    };
    let http = client(Duration::from_secs(25))?;
    lease.check()?;
    let value = json_response(
        http.post(TOKEN)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code", &code),
                ("code_verifier", &verifier),
                ("redirect_uri", &redirect),
                ("resource", RESOURCE),
            ])
            .send()
            .map_err(|_| anyhow!("ChatGPT sign-in exchange failed. Start again."))?,
    )?;
    let tokens = parse_tokens(&value, None)?;
    let identity = verify_identity(
        &tokens.id_token,
        &jwks(&http)?,
        &client_id,
        Some(&nonce),
        if previous.1.subject.is_empty() {
            None
        } else {
            Some(&previous.1.subject)
        },
    )?;
    let _file = store::lock(lease)?;
    let mut vault = store::load()?;
    let label = identity
        .email
        .clone()
        .unwrap_or_else(|| "ChatGPT account".into());
    vault.registration = Registration {
        client_id,
        subject: identity.sub,
        email: identity.email,
        tokens: Some(tokens),
    };
    lease.commit(|| store::save(&vault))?;
    Ok(status(
        "chatgpt",
        "connected",
        Some(label),
        "ChatGPT plan connected. Run a connection test to verify inference.",
    ))
}
pub(super) fn connection(lease: Lease) -> Result<ConnectionStatus> {
    let _file = store::lock(lease)?;
    let vault = store::load()?;
    Ok(if vault.registration.tokens.is_some() {
        status(
            "chatgpt",
            "connected",
            vault.registration.email,
            "ChatGPT sign-in saved. Run a connection test to verify current plan access.",
        )
    } else {
        status(
            "chatgpt",
            "disconnected",
            None,
            "Continue with ChatGPT to allow plan usage.",
        )
    })
}
fn access(lease: Lease) -> Result<String> {
    let _file = store::lock(lease)?;
    let mut vault = store::load()?;
    let registration = &mut vault.registration;
    let current = registration
        .tokens
        .as_ref()
        .ok_or_else(|| anyhow!("Connect ChatGPT in AI settings first."))?;
    if !has_permission(&current.scope) {
        bail!("ChatGPT plan usage has not been granted. Sign in again.");
    }
    if current.expires_at > store::now() + 90 {
        lease.check()?;
        return Ok(current.access_token.clone());
    }
    let http = client(Duration::from_secs(25))?;
    lease.check()?;
    let value = json_response(
        http.post(TOKEN)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", registration.client_id.as_str()),
                ("refresh_token", current.refresh_token.as_str()),
                ("resource", RESOURCE),
            ])
            .send()
            .map_err(|_| {
                anyhow!("Cannot renew ChatGPT access. Check your connection or sign in again.")
            })?,
    )?;
    let rotated = parse_tokens(&value, Some(current))?;
    if value.get("id_token").is_some() {
        verify_identity(
            &rotated.id_token,
            &jwks(&http)?,
            &registration.client_id,
            None,
            Some(&registration.subject),
        )?;
    }
    let access_token = rotated.access_token.clone();
    registration.tokens = Some(rotated);
    // A privacy change cancels text upload, but must not discard a refresh
    // token already rotated by the provider. A disconnect still blocks commit.
    lease.session_only().commit(|| store::save(&vault))?;
    lease.check()?;
    Ok(access_token)
}
pub(super) fn models(lease: Lease) -> Result<Vec<SubscriptionModel>> {
    let token = access(lease)?;
    let http = client(Duration::from_secs(20))?;
    lease.check()?;
    let value = json_response(
        http.get(format!("{RESOURCE}/models"))
            .bearer_auth(token)
            .send()
            .map_err(|_| anyhow!("Cannot list ChatGPT subscription models."))?,
    )?;
    let models = value["models"]
        .as_array()
        .ok_or_else(|| anyhow!("ChatGPT returned an invalid model catalog."))?
        .iter()
        .filter(|item| item["visibility"] == "list")
        .take(200)
        .filter_map(|item| {
            Some(SubscriptionModel {
                id: item["slug"].as_str()?.into(),
                name: item["display_name"].as_str()?.into(),
            })
        })
        .collect::<Vec<_>>();
    lease.check()?;
    if models.is_empty() {
        bail!("No ChatGPT models are available to this account.");
    }
    Ok(models)
}
pub(super) fn generate(
    lease: Lease,
    text: &str,
    instructions: &str,
    model: &str,
) -> Result<String> {
    if model.trim().is_empty() {
        bail!("Choose an available ChatGPT subscription model in AI settings.");
    }
    // Catalog validation is deliberately before any transcript upload.
    if !models(lease)?.iter().any(|m| m.id == model) {
        bail!("Selected ChatGPT model is unavailable. Refresh models in AI settings.");
    }
    let token = access(lease)?;
    let http = client(Duration::from_secs(45))?;
    lease.check()?;
    let request = http.post(format!("{RESOURCE}/responses")).bearer_auth(token).json(&json!({"model":model,"instructions":instructions,"input":[{"role":"user","content":text}],"store":false,"stream":true,"tools":[],"max_output_tokens":8192}));
    let response = lease.dispatch_text(|| {
        request
            .send()
            .map_err(|_| anyhow!("ChatGPT request failed or timed out."))
    })?;
    if !response.status().is_success() {
        return json_response(response)
            .and_then(|_| Err(anyhow!("ChatGPT did not return a stream.")));
    }
    let bytes = read_bounded(response, 1024 * 1024)?;
    let result = protocol::completed_text(&bytes)?;
    lease.check()?;
    Ok(result)
}
pub(super) fn disconnect(lease: Lease) -> Result<String> {
    let _file = store::lock(lease)?;
    let mut vault = store::load()?;
    let tokens = vault.registration.tokens.take();
    // Persist local sign-out before any network operation; never resurrect on failure.
    lease.commit(|| store::save(&vault))?;
    let Some(tokens) = tokens else {
        return Ok("Disconnected from Lectus.".into());
    };
    let revoke = || -> Result<()> {
        let http = client(Duration::from_secs(8))?;
        let discovery = json_response(
            http.get(format!("{ISSUER}/.well-known/openid-configuration"))
                .send()
                .map_err(|_| anyhow!("Cannot confirm remote revocation."))?,
        )?;
        let endpoint = discovery["revocation_endpoint"]
            .as_str()
            .ok_or_else(|| anyhow!("Cannot confirm remote revocation."))?;
        let url = reqwest::Url::parse(endpoint)
            .map_err(|_| anyhow!("Cannot confirm remote revocation."))?;
        if url.scheme() != "https" || url.host_str() != Some("auth.openai.com") {
            bail!("Invalid remote revocation endpoint.");
        }
        let response = http
            .post(url)
            .form(&[
                ("token", tokens.refresh_token.as_str()),
                ("token_type_hint", "refresh_token"),
                ("client_id", vault.registration.client_id.as_str()),
            ])
            .send()
            .map_err(|_| anyhow!("Cannot confirm remote revocation."))?;
        if response.status() != reqwest::StatusCode::OK {
            bail!("Cannot confirm remote revocation.");
        }
        Ok(())
    };
    for delay in [0, 150, 400] {
        lease.check()?;
        thread::sleep(Duration::from_millis(delay));
        if revoke().is_ok() {
            return Ok("Disconnected from Lectus and renewable ChatGPT session revoked.".into());
        }
    }
    Ok("Disconnected locally. Remote revocation was not confirmed; disconnect Lectus in ChatGPT Settings to revoke it there.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_rejects_wrong_state_client_and_duplicate_parameters() {
        assert_eq!(
            callback("/auth/callback?state=s&code=c&client_id=oaiapp_1", "s", "").unwrap(),
            ("c".into(), "oaiapp_1".into())
        );
        for target in [
            "/auth/callback?state=x&code=c&client_id=oaiapp_1",
            "/auth/callback?state=s&code=c&client_id=dynamic_agent_client",
            "/auth/callback?state=s&state=s&code=c&client_id=oaiapp_1",
            "/callback?state=s&code=c&client_id=oaiapp_1",
            "/auth/callback?state=s&error=access_denied",
        ] {
            assert!(callback(target, "s", "").is_err());
        }
        assert!(callback(
            "/auth/callback?state=s&code=c&client_id=oaiapp_2",
            "s",
            "oaiapp_1"
        )
        .is_err());
    }
    #[test]
    fn pkce_authorization_uses_exact_callback_and_stable_registration() {
        let url = authorization_url(
            "host",
            &Registration::default(),
            "http://127.0.0.1:1234/auth/callback",
            "state",
            "nonce",
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        )
        .unwrap();
        let query = url.query_pairs().collect::<HashMap<_, _>>();
        assert_eq!(query["client_id"], "dynamic_agent_client");
        assert_eq!(query["redirect_uri"], "http://127.0.0.1:1234/auth/callback");
        assert_eq!(query["agent_name_hint"], "Lectus");
        assert_eq!(
            query["code_challenge"],
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        let registration = Registration {
            client_id: "oaiapp_fixture".into(),
            ..Default::default()
        };
        let returning = authorization_url(
            "host",
            &registration,
            "http://127.0.0.1:4321/auth/callback",
            "state",
            "nonce",
            "verifier",
        )
        .unwrap();
        assert!(!returning
            .query_pairs()
            .any(|(key, _)| key == "agent_name_hint"));
    }
    #[test]
    fn refresh_rotates_atomically_and_revoked_scope_fails_closed() {
        let old = Tokens {
            access_token: "old-access".into(),
            refresh_token: "old-refresh".into(),
            id_token: "id".into(),
            scope: SCOPES.into(),
            expires_at: 1,
        };
        let value = json!({"access_token":"new-access","refresh_token":"new-refresh","token_type":"Bearer","expires_in":3600});
        let rotated = parse_tokens(&value, Some(&old)).unwrap();
        assert_eq!(rotated.refresh_token, "new-refresh");
        assert_eq!(rotated.access_token, "new-access");
        let mut revoked = value;
        revoked["scope"] = json!("openid email");
        assert!(parse_tokens(&revoked, Some(&old)).is_err());
    }
    #[test]
    fn signed_identity_binds_nonce_audience_issuer_expiry_and_subject() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use ed25519_dalek::{Signer, SigningKey};
        let signing = SigningKey::from_bytes(&[19; 32]);
        let jwks: JwkSet = serde_json::from_value(json!({"keys":[{"kty":"OKP","crv":"Ed25519","kid":"fixture","alg":"EdDSA","x":URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes())}]})).unwrap();
        let claims = json!({"iss":ISSUER,"aud":"oaiapp_fixture","sub":"person","iat":store::now(),"exp":store::now()+600,"nonce":"nonce"});
        let token = |claims: &Value| {
            let message = format!(
                "{}.{}",
                URL_SAFE_NO_PAD.encode(br#"{"alg":"EdDSA","kid":"fixture"}"#),
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).unwrap())
            );
            format!(
                "{message}.{}",
                URL_SAFE_NO_PAD.encode(signing.sign(message.as_bytes()).to_bytes())
            )
        };
        assert!(verify_identity(
            &token(&claims),
            &jwks,
            "oaiapp_fixture",
            Some("nonce"),
            Some("person")
        )
        .is_ok());
        assert!(verify_identity(&token(&claims), &jwks, "other", Some("nonce"), None).is_err());
        assert!(verify_identity(
            &token(&claims),
            &jwks,
            "oaiapp_fixture",
            Some("other"),
            None
        )
        .is_err());
        assert!(verify_identity(
            &token(&claims),
            &jwks,
            "oaiapp_fixture",
            None,
            Some("other")
        )
        .is_err());
        for (key, value) in [
            ("iss", json!("https://attacker.invalid")),
            ("exp", json!(1)),
        ] {
            let mut bad = claims.clone();
            bad[key] = value;
            assert!(
                verify_identity(&token(&bad), &jwks, "oaiapp_fixture", Some("nonce"), None)
                    .is_err()
            );
        }
        let mut tampered = token(&claims);
        tampered.push('a');
        assert!(verify_identity(&tampered, &jwks, "oaiapp_fixture", Some("nonce"), None).is_err());
    }
}
