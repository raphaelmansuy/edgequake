//! Password hashing service using Argon2.

use std::sync::Arc;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

use crate::config::AuthConfig;
use crate::error::AuthError;

/// Password hashing service using Argon2id.
#[derive(Clone)]
pub struct PasswordService {
    config: Arc<AuthConfig>,
}

impl PasswordService {
    /// Create a new password service with the given configuration.
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Create an Argon2 hasher with the configured parameters.
    fn argon2(&self) -> Result<Argon2<'_>, AuthError> {
        let params = Params::new(
            self.config.argon2_memory_cost,
            self.config.argon2_time_cost,
            self.config.argon2_parallelism,
            None,
        )
        .map_err(|e| AuthError::PasswordHashingFailed {
            reason: e.to_string(),
        })?;

        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }

    /// Hash a password using Argon2id.
    pub fn hash_password(&self, password: &str) -> Result<String, AuthError> {
        // Validate password
        self.validate_password_strength(password)?;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = self.argon2()?;

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHashingFailed {
                reason: e.to_string(),
            })?;

        Ok(password_hash.to_string())
    }

    /// Verify a password against a hash.
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AuthError> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| AuthError::PasswordHashingFailed {
                reason: format!("Invalid hash format: {}", e),
            })?;

        let argon2 = self.argon2()?;

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(AuthError::PasswordHashingFailed {
                reason: e.to_string(),
            }),
        }
    }

    /// Check if a password hash needs to be upgraded (re-hashed with new parameters).
    pub fn needs_rehash(&self, hash: &str) -> bool {
        let parsed = match PasswordHash::new(hash) {
            Ok(h) => h,
            Err(_) => return true,
        };

        // Check if algorithm is Argon2id
        if parsed.algorithm.as_str() != "argon2id" {
            return true;
        }

        // Check if parameters match current config
        if let (Some(m), Some(t), Some(p)) = (
            parsed.params.get_str("m"),
            parsed.params.get_str("t"),
            parsed.params.get_str("p"),
        ) {
            let current_m = m.parse::<u32>().unwrap_or(0);
            let current_t = t.parse::<u32>().unwrap_or(0);
            let current_p = p.parse::<u32>().unwrap_or(0);

            return current_m < self.config.argon2_memory_cost
                || current_t < self.config.argon2_time_cost
                || current_p < self.config.argon2_parallelism;
        }

        true
    }

    /// Validate password strength.
    pub fn validate_password_strength(&self, password: &str) -> Result<(), AuthError> {
        if password.len() < 8 {
            return Err(AuthError::WeakPassword {
                reason: "Password must be at least 8 characters".to_string(),
            });
        }

        if password.len() > 128 {
            return Err(AuthError::WeakPassword {
                reason: "Password must be at most 128 characters".to_string(),
            });
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        let score = [has_uppercase, has_lowercase, has_digit, has_special]
            .iter()
            .filter(|&&x| x)
            .count();

        if score < 3 {
            return Err(AuthError::WeakPassword {
                reason: "Password must contain at least 3 of: uppercase, lowercase, digit, special character".to_string(),
            });
        }

        // Check for common weak passwords
        let common_passwords = [
            "password",
            "12345678",
            "qwerty123",
            "admin123",
            "letmein",
            "welcome1",
        ];

        let lower_password = password.to_lowercase();
        for common in &common_passwords {
            if lower_password.contains(common) {
                return Err(AuthError::WeakPassword {
                    reason: "Password contains a common pattern".to_string(),
                });
            }
        }

        Ok(())
    }
}

impl std::fmt::Debug for PasswordService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PasswordService")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> AuthConfig {
        // Use lower parameters for faster tests
        AuthConfig::default().with_argon2_params(4096, 1, 1)
    }

    #[test]
    fn test_hash_password() {
        let service = PasswordService::new(test_config());
        let password = "SecureP@ssw0rd!";

        let hash = service.hash_password(password).unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(hash.len() > 50);
    }

    #[test]
    fn test_verify_password() {
        let service = PasswordService::new(test_config());
        let password = "SecureP@ssw0rd!";

        let hash = service.hash_password(password).unwrap();

        assert!(service.verify_password(password, &hash).unwrap());
        assert!(!service.verify_password("WrongPassword1!", &hash).unwrap());
    }

    #[test]
    fn test_weak_password_too_short() {
        let service = PasswordService::new(test_config());
        let result = service.hash_password("Short1!");
        assert!(matches!(result, Err(AuthError::WeakPassword { .. })));
    }

    #[test]
    fn test_weak_password_no_complexity() {
        let service = PasswordService::new(test_config());
        let result = service.hash_password("alllowercase");
        assert!(matches!(result, Err(AuthError::WeakPassword { .. })));
    }

    #[test]
    fn test_weak_password_common_pattern() {
        let service = PasswordService::new(test_config());
        let result = service.hash_password("mypassword123!");
        assert!(matches!(result, Err(AuthError::WeakPassword { .. })));
    }

    #[test]
    fn test_valid_strong_password() {
        let service = PasswordService::new(test_config());

        // These should all pass
        assert!(service.hash_password("Str0ng!Pass").is_ok());
        assert!(service.hash_password("MyC0mplexP@ss").is_ok());
        assert!(service.hash_password("ABC123!def").is_ok());
    }

    #[test]
    fn test_needs_rehash_with_old_params() {
        let weak_config = AuthConfig::default().with_argon2_params(1024, 1, 1);
        let weak_service = PasswordService::new(weak_config);

        let strong_config = AuthConfig::default().with_argon2_params(65536, 3, 4);
        let strong_service = PasswordService::new(strong_config);

        let hash = weak_service.hash_password("SecureP@ssw0rd!").unwrap();

        // Strong service should detect that hash needs upgrade
        assert!(strong_service.needs_rehash(&hash));
    }

    #[test]
    fn test_needs_rehash_with_current_params() {
        let config = AuthConfig::default().with_argon2_params(4096, 1, 1);
        let service = PasswordService::new(config);

        let hash = service.hash_password("SecureP@ssw0rd!").unwrap();

        // Same service should not need rehash
        assert!(!service.needs_rehash(&hash));
    }
}
