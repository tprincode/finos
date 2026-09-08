//! Test-issuer HS256 JWTs for goldens. Production uses the same shape with env issuer/audience/secret.
//! Live Auth0/Okta JWKS is not required to pass M9.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEST_ISSUER: &str = "http://finos.test/oidc";
pub const TEST_AUDIENCE: &str = "finos-api";
/// Loopback/golden secret only. Not a production IdP key.
pub const TEST_HS256_SECRET: &str = "finos-oidc-test-hs256-not-for-production";

#[derive(Clone)]
pub struct AuthConfig {
    pub issuer: String,
    pub audience: String,
    hs256_secret: String,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub user_sub: String,
    pub device_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    device_id: String,
    iss: String,
    aud: String,
    exp: usize,
    iat: usize,
}

impl AuthConfig {
    pub fn test_issuer() -> Self {
        Self {
            issuer: TEST_ISSUER.to_string(),
            audience: TEST_AUDIENCE.to_string(),
            hs256_secret: TEST_HS256_SECRET.to_string(),
        }
    }

    /// `OIDC_ISSUER` + `OIDC_AUDIENCE` + `OIDC_TEST_HS256_SECRET`, or the golden test issuer.
    pub fn from_env_or_test() -> Self {
        match (
            std::env::var("OIDC_ISSUER"),
            std::env::var("OIDC_AUDIENCE"),
            std::env::var("OIDC_TEST_HS256_SECRET"),
        ) {
            (Ok(issuer), Ok(audience), Ok(secret)) => Self {
                issuer,
                audience,
                hs256_secret: secret,
            },
            (Err(_), Err(_), Err(_)) => Self::test_issuer(),
            _ => panic!(
                "OIDC_ISSUER, OIDC_AUDIENCE, and OIDC_TEST_HS256_SECRET must be set together \
                 (or all omitted to use the test issuer)"
            ),
        }
    }

    pub fn mint_token(&self, user_sub: &str, device_id: &str) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as usize;
        let claims = Claims {
            sub: user_sub.to_string(),
            device_id: device_id.to_string(),
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            iat: now,
            exp: now + 3600,
        };
        encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(self.hs256_secret.as_bytes()),
        )
        .map_err(|e| e.to_string())
    }

    pub fn authenticate(&self, bearer: &str) -> Result<Identity, String> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.issuer]);
        validation.set_audience(&[&self.audience]);
        validation.validate_exp = true;
        let token = decode::<Claims>(
            bearer,
            &DecodingKey::from_secret(self.hs256_secret.as_bytes()),
            &validation,
        )
        .map_err(|e| e.to_string())?;
        if token.claims.sub.is_empty() || token.claims.device_id.is_empty() {
            return Err("missing sub or device_id".to_string());
        }
        Ok(Identity {
            user_sub: token.claims.sub,
            device_id: token.claims.device_id,
        })
    }
}

pub fn mint_test_token(user_sub: &str, device_id: &str) -> String {
    AuthConfig::test_issuer()
        .mint_token(user_sub, device_id)
        .expect("test issuer mint")
}
