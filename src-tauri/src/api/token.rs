use super::ApiError;

pub fn generate() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes)
        .map_err(|_| ApiError::internal("Could not generate an API token."))?;
    Ok(bs58::encode(bytes).into_string())
}

pub fn valid(token: &str) -> bool {
    bs58::decode(token).into_vec().map(|v| v.len() == 32).unwrap_or(false)
}

/// Fixed-width comparison after decoding; no early exit on the first differing
/// secret byte. black_box prevents the reduction becoming a short-circuit memcmp.
pub fn matches(header: Option<&str>, expected: &str) -> bool {
    let Some(candidate) = header.and_then(|h| h.strip_prefix("Bearer ")) else { return false };
    if candidate.len() > 44 || expected.is_empty() { return false; }
    let Ok(actual) = bs58::decode(candidate).into_vec() else { return false };
    let Ok(expected) = bs58::decode(expected).into_vec() else { return false };
    if actual.len() != 32 || expected.len() != 32 { return false; }
    let mut difference = 0u8;
    for index in 0..32 {
        difference |= std::hint::black_box(actual[index] ^ expected[index]);
    }
    std::hint::black_box(difference) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_rejects_missing_malformed_short_and_wrong_tokens() {
        let token = generate().unwrap();
        assert!(valid(&token));
        assert!(matches(Some(&format!("Bearer {token}")), &token));
        for bad in [None, Some(""), Some("Basic abc"), Some("Bearer 123"), Some("Bearer !!!")] {
            assert!(!matches(bad, &token));
        }
        assert!(!matches(Some(&format!("Bearer {}", generate().unwrap())), &token));
    }
}
