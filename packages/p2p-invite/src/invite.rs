//! Invite code encoder, decoder, and secure memory representation
//!
//! Copyright (c) 2026 AstralRinth contributors / Gradle authors (MIT License).

use anyhow::{Context, Result, bail};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use iroh::EndpointAddr;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const INVITE_PREFIX: &str = "gmc1_";
pub const INVITE_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
pub struct InvitePayload {
    pub v: u32,
    pub token: String,
    pub addr: EndpointAddr,
}

#[derive(Clone)]
pub struct InviteCode {
    payload: InvitePayload,
    cached_raw: String,
}

impl InviteCode {
    pub fn new(addr: EndpointAddr, token: impl Into<String>) -> Result<Self> {
        let payload = InvitePayload {
            v: INVITE_VERSION,
            token: token.into(),
            addr,
        };
        let json = serde_json::to_vec(&payload)
            .context("serializing invite payload to JSON")?;
        let encoded = URL_SAFE_NO_PAD.encode(&json);
        let cached_raw = format!("{INVITE_PREFIX}{encoded}");
        Ok(Self {
            payload,
            cached_raw,
        })
    }

    pub fn generate_random_token() -> String {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];
        rng.fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    pub fn decode(code: &str) -> Result<Self> {
        let trimmed = code.trim();
        if !trimmed.starts_with(INVITE_PREFIX) {
            bail!("Invalid invite code: missing '{INVITE_PREFIX}' prefix");
        }

        let b64_payload = &trimmed[INVITE_PREFIX.len()..];
        let json_bytes = URL_SAFE_NO_PAD
            .decode(b64_payload)
            .context("decoding base64 invite payload")?;

        let payload: InvitePayload = serde_json::from_slice(&json_bytes)
            .context("deserializing invite payload JSON")?;

        if payload.v != INVITE_VERSION {
            bail!("Unsupported invite code version: {}", payload.v);
        }

        if payload.token.is_empty() {
            bail!("Invite code contains empty authentication token");
        }

        Ok(Self {
            payload,
            cached_raw: trimmed.to_string(),
        })
    }

    /// Access the raw string to copy to the clipboard.
    /// NEVER log or write this value to crash reports or persistent storage.
    pub fn raw_code(&self) -> &str {
        &self.cached_raw
    }

    pub fn token(&self) -> &str {
        &self.payload.token
    }

    pub fn addr(&self) -> &EndpointAddr {
        &self.payload.addr
    }

    pub fn endpoint_id(&self) -> iroh::EndpointId {
        self.payload.addr.id
    }
}

// Ensure the token is redacted in Debug and Display to prevent accidental leakage in logs
impl fmt::Debug for InviteCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InviteCode({}***[REDACTED]***, id: {})",
            INVITE_PREFIX, self.payload.addr.id
        )
    }
}

impl fmt::Display for InviteCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}***[REDACTED]***", INVITE_PREFIX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::{Endpoint, endpoint::presets};

    #[tokio::test]
    async fn test_invite_code_encode_decode() {
        let endpoint = Endpoint::builder(presets::N0)
            .alpns(vec![b"test/alpn".to_vec()])
            .bind()
            .await
            .unwrap();

        let addr = endpoint.addr();
        let token = InviteCode::generate_random_token();
        let invite = InviteCode::new(addr.clone(), token.clone()).unwrap();

        let raw = invite.raw_code();
        assert!(raw.starts_with(INVITE_PREFIX));

        let decoded = InviteCode::decode(raw).unwrap();
        assert_eq!(decoded.token(), token);
        assert_eq!(decoded.endpoint_id(), addr.id);

        // Verify redaction in Display / Debug
        let display_str = format!("{invite}");
        assert!(!display_str.contains(&token));
        assert!(display_str.contains("REDACTED"));

        let debug_str = format!("{invite:?}");
        assert!(!debug_str.contains(&token));
        assert!(debug_str.contains("REDACTED"));

        endpoint.close().await;
    }

    #[test]
    fn test_invalid_prefix() {
        let err = InviteCode::decode("invalid_code_123");
        assert!(err.is_err());
    }

    #[test]
    fn test_tampered_payload() {
        let err =
            InviteCode::decode(&format!("{}invalid_base64_???", INVITE_PREFIX));
        assert!(err.is_err());
    }
}
