//! P2P Tunnel Protocol definitions and framing constants
//!
//! Copyright (c) 2026 Gradle contributors / Gradle authors (MIT License).

use anyhow::{Result, bail};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const ALPN: &[u8] = b"gradle/mc-tunnel/1";
pub const PROTOCOL_MAGIC: &[u8; 4] = b"GMCT";
pub const PROTOCOL_VERSION: u16 = 1;

pub const HANDSHAKE_KIND_VERIFY: u8 = 0x01;
pub const HANDSHAKE_KIND_TUNNEL: u8 = 0x02;

pub const HANDSHAKE_RESP_OK: u8 = 0x00;
pub const HANDSHAKE_RESP_REJECTED: u8 = 0x01;
pub const HANDSHAKE_RESP_REVOKED: u8 = 0x02;
pub const HANDSHAKE_RESP_MAX_PEERS: u8 = 0x03;

pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub const DEFAULT_MAX_PEERS: usize = 8;
pub const DEFAULT_MAX_STREAMS_PER_PEER: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeKind {
    Verify,
    Tunnel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamHandshake {
    pub version: u16,
    pub kind: HandshakeKind,
    pub token: String,
}

impl StreamHandshake {
    pub fn new(kind: HandshakeKind, token: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            kind,
            token: token.into(),
        }
    }

    pub async fn write_to<W: AsyncWrite + Unpin>(
        &self,
        writer: &mut W,
    ) -> Result<()> {
        writer.write_all(PROTOCOL_MAGIC).await?;
        writer.write_u16(self.version).await?;
        let kind_byte = match self.kind {
            HandshakeKind::Verify => HANDSHAKE_KIND_VERIFY,
            HandshakeKind::Tunnel => HANDSHAKE_KIND_TUNNEL,
        };
        writer.write_u8(kind_byte).await?;
        let token_bytes = self.token.as_bytes();
        let len = u16::try_from(token_bytes.len())?;
        writer.write_u16(len).await?;
        writer.write_all(token_bytes).await?;
        writer.flush().await?;
        Ok(())
    }

    pub async fn read_from<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).await?;
        if &magic != PROTOCOL_MAGIC {
            bail!("Invalid protocol magic: {:?}", magic);
        }

        let version = reader.read_u16().await?;
        if version != PROTOCOL_VERSION {
            bail!("Unsupported tunnel protocol version: {version}");
        }

        let kind_byte = reader.read_u8().await?;
        let kind = match kind_byte {
            HANDSHAKE_KIND_VERIFY => HandshakeKind::Verify,
            HANDSHAKE_KIND_TUNNEL => HandshakeKind::Tunnel,
            other => bail!("Unknown handshake kind: {other}"),
        };

        let token_len = reader.read_u16().await? as usize;
        if token_len > 1024 {
            bail!("Handshake token length exceeded maximum: {token_len}");
        }

        let mut token_buf = vec![0u8; token_len];
        reader.read_exact(&mut token_buf).await?;
        let token = String::from_utf8(token_buf)?;

        Ok(Self {
            version,
            kind,
            token,
        })
    }
}
