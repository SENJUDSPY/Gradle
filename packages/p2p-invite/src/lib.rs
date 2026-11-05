//! P2P Invite & Tunneling crate for Gradle / Minecraft Java 26.3
//!
//! Provides peer-to-peer game port tunneling via iroh QUIC with invite codes,
//! automatic NAT traversal / relay fallback, host approval, and token revocation.
//!
//! Copyright (c) 2026 AstralRinth contributors / Gradle authors (MIT License).

pub mod host;
pub mod invite;
pub mod joiner;
pub mod protocol;

pub use host::{ConnectedPeer, HostEvent, HostHandle, HostOptions, start_host};
pub use invite::InviteCode;
pub use joiner::{JoinHandle, JoinOptions, TunnelStatus, join};
pub type ConnectionStatus = TunnelStatus;
