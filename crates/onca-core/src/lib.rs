//! # onca-core
//!
//! A small, MIT-licensed Solana substrate that compiles for `wasm32-wasip2`
//! inside a ZeroClaw WIT component — *without* `solana-sdk` or `solana-client`,
//! which do not build cleanly there.
//!
//! It carries no I/O and no wasm dependency. Plugins import it by path, get:
//!
//! - [`pubkey::Pubkey`] — parse/validate/render 32-byte addresses (base58).
//! - [`rpc`] — JSON-RPC 2.0 request building + envelope handling behind the
//!   [`rpc::RpcTransport`] trait, so the whole RPC layer is host-testable.
//! - [`shape`] — amount rendering/parsing and output clamping, so plugins return
//!   the ~200 tokens the model needs, not 40KB of RPC JSON.
//! - [`error::CoreError`] — a flat, secret-free error type safe to surface.
//!
//! Each plugin supplies the actual HTTP by implementing `RpcTransport` with the
//! blocking `waki` client on wasm. See `plugins/*/src/lib.rs`.

pub mod error;
pub mod pubkey;
pub mod rpc;
pub mod shape;

pub use error::{CoreError, Result};
pub use pubkey::Pubkey;
pub use rpc::{RpcClient, RpcTransport};
