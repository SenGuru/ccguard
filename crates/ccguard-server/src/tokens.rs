use rand::RngCore;
use sha2::{Digest, Sha256};

/// SHA-256 hex of a token string. Deterministic; what we store and look up by.
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Generate a new ingest token. Returns (plaintext, hash). The plaintext is shown
/// to the caller exactly once; only the hash is persisted.
pub fn generate_token() -> (String, String) {
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = format!("ccg_{}", hex::encode(bytes));
    let hash = hash_token(&token);
    (token, hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_token("ccg_abc"), hash_token("ccg_abc"));
        assert_ne!(hash_token("ccg_abc"), hash_token("ccg_xyz"));
        assert_eq!(hash_token("ccg_abc").len(), 64); // sha256 hex
    }

    #[test]
    fn generate_returns_prefixed_token_with_matching_hash() {
        let (token, hash) = generate_token();
        assert!(token.starts_with("ccg_"));
        assert_eq!(token.len(), 4 + 48); // "ccg_" + 24 bytes hex
        assert_eq!(hash, hash_token(&token));
    }

    #[test]
    fn two_tokens_differ() {
        assert_ne!(generate_token().0, generate_token().0);
    }
}
