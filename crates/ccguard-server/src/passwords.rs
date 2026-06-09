use bcrypt::{hash, verify, DEFAULT_COST};

/// Hash a password with bcrypt (random per-hash salt).
pub fn hash_password(password: &str) -> String {
    hash(password, DEFAULT_COST).expect("bcrypt hash")
}

/// Verify a password against a bcrypt hash. Returns false on any mismatch or parse error.
pub fn verify_password(password: &str, hashed: &str) -> bool {
    verify(password, hashed).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let h = hash_password("hunter2");
        assert!(verify_password("hunter2", &h));
        assert!(!verify_password("wrong", &h));
    }

    #[test]
    fn hashes_are_salted_and_differ() {
        assert_ne!(hash_password("same"), hash_password("same"));
    }
}
