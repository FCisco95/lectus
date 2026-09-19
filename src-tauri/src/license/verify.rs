//! Wallet-ownership proof: ed25519 over a one-shot nonce.
//!
//! Signing a message is free and cannot move funds — it only shows the person
//! at the keyboard controls the private key for the wallet they claim.

use anyhow::{anyhow, bail, Result};
use ed25519_dalek::{Signature, VerifyingKey};

/// Text the wallet is asked to sign. Written to be readable inside Phantom's
/// confirmation sheet, so nobody signs something they can't parse.
pub fn challenge_message(nonce: &str) -> String {
    format!(
        "Lectus — verify ORGANIC or Mycel holdings\n\n\
         Signing proves this wallet is yours. It is not a transaction: no SOL, \
         no tokens and no approvals are involved.\n\n\
         Nonce: {nonce}"
    )
}

/// A fresh 32-byte random nonce, base58-encoded.
///
/// Sourced from the OS CSPRNG via `getrandom` (the same source `ed25519-dalek`
/// uses for keygen) rather than a time-seeded PRNG, so a nonce cannot be
/// guessed from the clock.
pub fn new_nonce() -> String {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("OS randomness unavailable");
    bs58::encode(bytes).into_string()
}

/// Check that `signature_b58` is `pubkey_b58`'s signature over the challenge
/// built from `nonce`.
pub fn verify_signature(pubkey_b58: &str, signature_b58: &str, nonce: &str) -> Result<()> {
    let key_bytes: [u8; 32] = bs58::decode(pubkey_b58)
        .into_vec()
        .map_err(|e| anyhow!("public key is not base58: {e}"))?
        .try_into()
        .map_err(|_| anyhow!("public key is not 32 bytes"))?;

    let sig_bytes: [u8; 64] = bs58::decode(signature_b58)
        .into_vec()
        .map_err(|e| anyhow!("signature is not base58: {e}"))?
        .try_into()
        .map_err(|_| anyhow!("signature is not 64 bytes"))?;

    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| anyhow!("public key is not a valid ed25519 point: {e}"))?;
    let signature = Signature::from_bytes(&sig_bytes);

    // verify_strict rejects small-order / torsion-component keys, which the
    // permissive check would let through.
    if key
        .verify_strict(challenge_message(nonce).as_bytes(), &signature)
        .is_err()
    {
        bail!("signature does not match this wallet");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn keypair() -> SigningKey {
        // Fixed seed: the test must not depend on system randomness.
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn sign(nonce: &str) -> (String, String) {
        let key = keypair();
        let sig = key.sign(challenge_message(nonce).as_bytes());
        (
            bs58::encode(key.verifying_key().to_bytes()).into_string(),
            bs58::encode(sig.to_bytes()).into_string(),
        )
    }

    #[test]
    fn accepts_a_real_signature() {
        let nonce = "TestNonce111";
        let (pubkey, sig) = sign(nonce);
        assert!(verify_signature(&pubkey, &sig, nonce).is_ok());
    }

    #[test]
    fn rejects_a_replay_under_a_different_nonce() {
        let (pubkey, sig) = sign("NonceA");
        assert!(verify_signature(&pubkey, &sig, "NonceB").is_err());
    }

    #[test]
    fn rejects_another_wallets_pubkey() {
        let nonce = "TestNonce111";
        let (_, sig) = sign(nonce);
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let other_pubkey = bs58::encode(other.verifying_key().to_bytes()).into_string();
        assert!(verify_signature(&other_pubkey, &sig, nonce).is_err());
    }

    #[test]
    fn rejects_malformed_input() {
        let nonce = "TestNonce111";
        let (pubkey, sig) = sign(nonce);
        assert!(verify_signature("not base58 !!", &sig, nonce).is_err());
        assert!(verify_signature(&pubkey, "not base58 !!", nonce).is_err());
        assert!(verify_signature("TooShort", &sig, nonce).is_err());
    }

    #[test]
    fn nonces_do_not_repeat() {
        let a = new_nonce();
        let b = new_nonce();
        assert_ne!(a, b);
        assert!(bs58::decode(&a).into_vec().unwrap().len() == 32);
    }
}
