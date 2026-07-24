//! # onca-core
//!
//! A small, MIT-licensed Solana substrate that compiles for `wasm32-wasip2`
//! inside a ZeroClaw WIT component. It stays dependency-light on purpose: the
//! modular `solana-*` crates compile for the target now, but this hand-rolled
//! core is minimal and devnet-verified (see `docs/wasm-notes.md`).
//!
//! It carries no I/O and no wasm dependency. Plugins import it by path, get:
//!
//! - [`pubkey::Pubkey`] — parse/validate/render 32-byte addresses (base58).
//! - [`rpc`] — JSON-RPC 2.0 request building + envelope handling behind the
//!   [`rpc::RpcTransport`] trait, so the whole RPC layer is host-testable.
//! - [`shape`] — amount rendering/parsing and output clamping, so plugins return
//!   the ~200 tokens the model needs, not 40KB of RPC JSON.
//! - [`tx`] — hand-rolled Solana transaction assembly (no `solana-sdk`): the
//!   compact-u16 prefix, legacy message compilation, base64, the SPL Memo
//!   instruction, and the durable-nonce advance for approval-gated flows.
//! - [`mesh`] — aggregate many device attestations into one manipulation-resistant
//!   oracle value (median, outliers dropped) for a prediction market to settle on.
//! - [`error::CoreError`] — a flat, secret-free error type safe to surface.
//!
//! Each plugin supplies the actual HTTP by implementing `RpcTransport` with the
//! blocking `waki` client on wasm. See `plugins/*/src/lib.rs`.

pub mod error;
pub mod mesh;
pub mod pubkey;
pub mod rpc;
pub mod shape;
pub mod tx;

pub use error::{CoreError, Result};
pub use pubkey::Pubkey;
pub use rpc::{RpcClient, RpcTransport};
