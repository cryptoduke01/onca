//! Error type shared across the core. No `std::error::Error` boxing so the
//! surface stays small and `no_std`-friendly in spirit (we still use `alloc`
//! via `std::string::String`, which wasm32-wasip2 provides).

use core::fmt;

/// Everything that can go wrong assembling a request, decoding an address, or
/// interpreting an RPC response. Kept as flat data (never holds a secret) so it
/// is safe to surface into a `tool-result` error string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    /// A base58 string was malformed or decoded to the wrong length.
    Base58(String),
    /// A pubkey was not 32 bytes.
    BadPubkey(String),
    /// The RPC transport failed (network, TLS, non-2xx). Carries a message, not
    /// a secret — the URL/api-key is never included.
    Transport(String),
    /// The RPC response was not the JSON shape we expected.
    Rpc(String),
    /// The RPC returned a JSON-RPC `error` object.
    RpcError { code: i64, message: String },
    /// A caller argument was invalid (out of range, missing, etc.).
    Invalid(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::Base58(m) => write!(f, "base58: {m}"),
            CoreError::BadPubkey(m) => write!(f, "invalid pubkey: {m}"),
            CoreError::Transport(m) => write!(f, "rpc transport: {m}"),
            CoreError::Rpc(m) => write!(f, "rpc response: {m}"),
            CoreError::RpcError { code, message } => {
                write!(f, "rpc error {code}: {message}")
            }
            CoreError::Invalid(m) => write!(f, "invalid argument: {m}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Convenient result alias.
pub type Result<T> = core::result::Result<T, CoreError>;
