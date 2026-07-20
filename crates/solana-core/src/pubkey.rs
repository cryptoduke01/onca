//! A 32-byte Ed25519 public key with base58 text encoding — the Solana address.
//!
//! We deliberately do NOT pull in `solana-sdk` (it does not compile cleanly for
//! `wasm32-wasip2` inside a WIT component). This is the minimal, dependency-light
//! replacement the plugins need: parse, validate, render, and compare.

use crate::error::{CoreError, Result};

/// A Solana public key: 32 raw bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pubkey([u8; 32]);

impl Pubkey {
    /// The all-zero pubkey — also the SystemProgram / "null" address `111…111`.
    pub const ZERO: Pubkey = Pubkey([0u8; 32]);

    /// Wrap raw bytes.
    pub const fn new(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }

    /// Parse a base58 address. Rejects anything that does not decode to exactly
    /// 32 bytes — the single most common source of "agent hallucinated an
    /// address" bugs, caught here instead of at the RPC.
    pub fn from_base58(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.is_empty() {
            return Err(CoreError::BadPubkey("empty string".into()));
        }
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|e| CoreError::Base58(e.to_string()))?;
        let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
            CoreError::BadPubkey(format!("decoded to {} bytes, expected 32", bytes.len()))
        })?;
        Ok(Pubkey(arr))
    }

    /// Raw bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    /// Base58 text form.
    pub fn to_base58(&self) -> String {
        bs58::encode(self.0).into_string()
    }

    /// True for the system/default all-zero key.
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl core::fmt::Display for Pubkey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_base58())
    }
}

impl core::str::FromStr for Pubkey {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self> {
        Pubkey::from_base58(s)
    }
}

/// A few well-known program addresses the plugins reference by name so we never
/// hardcode a base58 literal at each call site.
pub mod known {
    /// SPL Token program.
    pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    /// SPL Token-2022 program.
    pub const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
    /// Associated Token Account program.
    pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
    /// System program (also the all-ones base58 `111…`).
    pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
    /// Memo program v2 — used for invoice references.
    pub const MEMO_PROGRAM: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";
    /// Native mint (wrapped SOL).
    pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
    /// Circle USDC mainnet mint.
    pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    /// Tether USDT mainnet mint.
    pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
}
