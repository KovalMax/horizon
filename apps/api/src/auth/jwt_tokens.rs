use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use openssl::{hash::MessageDigest, pkey::PKey, sign::Signer};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{models::AccessPrincipal, token_service::TokenService};

pub struct JwtTokenService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    code_key: Vec<u8>,
    issuer: String,
    audience: String,
    key_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccessClaims {
    iss: String,
    aud: String,
    sub: String,
    sid: String,
    iat: i64,
    exp: i64,
    token_use: String,
}

impl JwtTokenService {
    pub fn new(secret: &str, issuer: String, audience: String, key_id: String) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            code_key: secret.as_bytes().to_vec(),
            issuer,
            audience,
            key_id,
        }
    }
}

impl TokenService for JwtTokenService {
    fn hash_sign_in_code(&self, challenge: Uuid, code: &str) -> Result<Vec<u8>, String> {
        let digest = || -> Result<Vec<u8>, openssl::error::ErrorStack> {
            let key = PKey::hmac(&self.code_key)?;
            let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
            signer.update(b"horizon:sign-in-code:v1:")?;
            signer.update(challenge.as_bytes())?;
            signer.update(code.as_bytes())?;
            signer.sign_to_vec()
        };
        digest().map_err(|_| "could not hash sign-in code".to_owned())
    }

    fn issue_access_token(&self, user_id: Uuid, session_id: Uuid) -> Result<String, String> {
        let now = Utc::now().timestamp();
        let claims = AccessClaims {
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            sub: user_id.to_string(),
            sid: session_id.to_string(),
            iat: now,
            exp: now + super::models::ACCESS_TOKEN_TTL_SECONDS,
            token_use: "access".to_owned(),
        };
        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.key_id.clone());
        header.typ = Some("at+jwt".to_owned());
        encode(&header, &claims, &self.encoding_key)
            .map_err(|_| "could not issue access token".to_owned())
    }

    fn verify_access_token(&self, token: &str) -> Option<AccessPrincipal> {
        let header = decode_header(token).ok()?;
        if header.alg != Algorithm::HS256
            || header.kid.as_deref() != Some(self.key_id.as_str())
            || header.typ.as_deref() != Some("at+jwt")
        {
            return None;
        }
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "exp", "iat"]);
        validation.leeway = 30;
        let claims = decode::<AccessClaims>(token, &self.decoding_key, &validation)
            .ok()?
            .claims;
        if claims.token_use != "access" || claims.iat > Utc::now().timestamp() + 30 {
            return None;
        }
        Some(AccessPrincipal {
            user_id: Uuid::parse_str(&claims.sub).ok()?,
            session_id: Uuid::parse_str(&claims.sid).ok()?,
        })
    }

    fn create_secret(&self) -> String {
        let mut bytes = [0_u8; 32];
        OsRng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn hash_secret(&self, secret: &str) -> Vec<u8> {
        Sha256::digest(secret.as_bytes()).to_vec()
    }
}
