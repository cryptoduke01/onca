//! Hand-rolled Solana transaction assembly. No `solana-sdk`, no `solana-client`.
//!
//! The bounty is explicit that the standard stack does not compile for a
//! `wasm32-wasip2` WIT component, and that you should assemble transactions from
//! `bs58` / `borsh` / hand-rolled instruction encoding. This module is that: the
//! compact-u16 length prefix, the legacy message layout, the unsigned-transaction
//! wrapper, base64, the SPL Memo instruction, and the durable-nonce advance.
//!
//! Everything here is pure and host-tested against RFC 4648 (base64) and the
//! documented compact-u16 vectors, plus structural checks on the message layout.
//! A plugin builds an [`Instruction`] list, calls [`compile_message`], and hands
//! the [`unsigned_transaction_base64`] to the host or a human to sign.

use crate::pubkey::Pubkey;

// ─────────────────────────────────────────────────────────────────────────
// compact-u16 (ShortVec): 1–3 bytes, 7 bits per byte, high bit = continue.
// ─────────────────────────────────────────────────────────────────────────

/// Encode a length as Solana's compact-u16 (ShortVec) prefix.
pub fn encode_len(mut len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(3);
    loop {
        let mut byte = (len & 0x7f) as u8;
        len >>= 7;
        if len == 0 {
            out.push(byte);
            break;
        }
        byte |= 0x80;
        out.push(byte);
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// base64 (RFC 4648, standard alphabet, padded).
// ─────────────────────────────────────────────────────────────────────────

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard, padded base64. Used to return an unsigned transaction as text.
pub fn base64(input: &[u8]) -> String {
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[(n >> 18 & 63) as usize] as char);
        out.push(B64[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6 & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[(n & 63) as usize] as char } else { '=' });
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────
// Instruction.
// ─────────────────────────────────────────────────────────────────────────

/// One account referenced by an instruction, with its signer/writable roles.
#[derive(Debug, Clone)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn signer_writable(pubkey: Pubkey) -> Self {
        AccountMeta { pubkey, is_signer: true, is_writable: true }
    }
    pub fn signer_readonly(pubkey: Pubkey) -> Self {
        AccountMeta { pubkey, is_signer: true, is_writable: false }
    }
    pub fn writable(pubkey: Pubkey) -> Self {
        AccountMeta { pubkey, is_signer: false, is_writable: true }
    }
    pub fn readonly(pubkey: Pubkey) -> Self {
        AccountMeta { pubkey, is_signer: false, is_writable: false }
    }
}

/// A single instruction: which program, which accounts, and the data bytes.
#[derive(Debug, Clone)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

// ─────────────────────────────────────────────────────────────────────────
// Legacy message compilation.
// ─────────────────────────────────────────────────────────────────────────

/// A compiled, serialized legacy message plus how many signatures it needs.
#[derive(Debug, Clone)]
pub struct CompiledMessage {
    pub bytes: Vec<u8>,
    pub num_required_signatures: u8,
}

// Internal: an account being collected during compilation.
struct Acc {
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}

/// Compile instructions into a serialized legacy message.
///
/// `fee_payer` is always the first account, a writable signer. Every other
/// account is deduplicated (its signer/writable roles are OR-ed across every
/// use), then ordered the way Solana requires: writable signers, readonly
/// signers, writable non-signers, readonly non-signers. Program ids come in as
/// readonly non-signers.
pub fn compile_message(
    fee_payer: &Pubkey,
    recent_blockhash: [u8; 32],
    instructions: &[Instruction],
) -> CompiledMessage {
    let mut accs: Vec<Acc> = vec![Acc {
        pubkey: *fee_payer,
        is_signer: true,
        is_writable: true,
    }];

    let mut push = |pubkey: Pubkey, is_signer: bool, is_writable: bool| {
        if let Some(a) = accs.iter_mut().find(|a| a.pubkey == pubkey) {
            a.is_signer |= is_signer;
            a.is_writable |= is_writable;
        } else {
            accs.push(Acc { pubkey, is_signer, is_writable });
        }
    };

    for ix in instructions {
        for m in &ix.accounts {
            push(m.pubkey, m.is_signer, m.is_writable);
        }
        // A program id is always a readonly, non-signer account.
        push(ix.program_id, false, false);
    }

    // Stable ordering into the four buckets. The fee payer stays at index 0.
    let fee = accs.remove(0);
    let mut ws: Vec<Acc> = vec![fee];
    let mut rs: Vec<Acc> = Vec::new();
    let mut wn: Vec<Acc> = Vec::new();
    let mut rn: Vec<Acc> = Vec::new();
    for a in accs {
        match (a.is_signer, a.is_writable) {
            (true, true) => ws.push(a),
            (true, false) => rs.push(a),
            (false, true) => wn.push(a),
            (false, false) => rn.push(a),
        }
    }
    let num_required_signatures = (ws.len() + rs.len()) as u8;
    let num_readonly_signed = rs.len() as u8;
    let num_readonly_unsigned = rn.len() as u8;

    let ordered: Vec<Pubkey> = ws
        .into_iter()
        .chain(rs)
        .chain(wn)
        .chain(rn)
        .map(|a| a.pubkey)
        .collect();

    let index_of = |pk: &Pubkey| ordered.iter().position(|k| k == pk).unwrap() as u8;

    let mut bytes = Vec::new();
    // header
    bytes.push(num_required_signatures);
    bytes.push(num_readonly_signed);
    bytes.push(num_readonly_unsigned);
    // account keys
    bytes.extend(encode_len(ordered.len()));
    for pk in &ordered {
        bytes.extend_from_slice(&pk.to_bytes());
    }
    // recent blockhash
    bytes.extend_from_slice(&recent_blockhash);
    // instructions
    bytes.extend(encode_len(instructions.len()));
    for ix in instructions {
        bytes.push(index_of(&ix.program_id));
        bytes.extend(encode_len(ix.accounts.len()));
        for m in &ix.accounts {
            bytes.push(index_of(&m.pubkey));
        }
        bytes.extend(encode_len(ix.data.len()));
        bytes.extend_from_slice(&ix.data);
    }

    CompiledMessage { bytes, num_required_signatures }
}

/// Wrap a compiled message as a wire transaction with empty (zeroed) signature
/// slots, base64-encoded. This is the unsigned transaction the host or a human
/// signs: the signature array is present and correctly sized, ready for the
/// signer to fill in, so a wallet can deserialize it directly.
pub fn unsigned_transaction_base64(msg: &CompiledMessage) -> String {
    let n = msg.num_required_signatures as usize;
    let mut tx = Vec::with_capacity(1 + n * 64 + msg.bytes.len());
    tx.extend(encode_len(n));
    tx.extend(std::iter::repeat(0u8).take(n * 64));
    tx.extend_from_slice(&msg.bytes);
    base64(&tx)
}

// ─────────────────────────────────────────────────────────────────────────
// SPL Memo instruction.
// ─────────────────────────────────────────────────────────────────────────

/// Build an SPL Memo (v2) instruction carrying `memo` as UTF-8 data. Each pubkey
/// in `signers` is attached as a readonly signer, so the memo is attributed to
/// (and must be signed by) that account.
pub fn memo_instruction(memo: &str, signers: &[Pubkey]) -> Instruction {
    let program_id = Pubkey::from_base58(crate::pubkey::known::MEMO_PROGRAM).unwrap();
    Instruction {
        program_id,
        accounts: signers.iter().map(|s| AccountMeta::signer_readonly(*s)).collect(),
        data: memo.as_bytes().to_vec(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Durable nonce: SystemProgram AdvanceNonceAccount (instruction index 4).
// ─────────────────────────────────────────────────────────────────────────

/// Build the `AdvanceNonceAccount` instruction. When a transaction uses a
/// durable nonce, this must be its FIRST instruction, and the message's
/// recent-blockhash field must be the nonce value stored in `nonce_account`.
/// That is how an approval-gated transaction survives the five minutes between
/// the agent building it and a human signing it (bounty trap #1).
pub fn advance_nonce_instruction(nonce_account: &Pubkey, authority: &Pubkey) -> Instruction {
    let system = Pubkey::from_base58(crate::pubkey::known::SYSTEM_PROGRAM).unwrap();
    let recent_blockhashes =
        Pubkey::from_base58(crate::pubkey::known::RECENT_BLOCKHASHES_SYSVAR).unwrap();
    Instruction {
        program_id: system,
        accounts: vec![
            AccountMeta::writable(*nonce_account),
            AccountMeta::readonly(recent_blockhashes),
            AccountMeta::signer_readonly(*authority),
        ],
        data: 4u32.to_le_bytes().to_vec(), // AdvanceNonceAccount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_u16_known_vectors() {
        assert_eq!(encode_len(0), vec![0x00]);
        assert_eq!(encode_len(5), vec![0x05]);
        assert_eq!(encode_len(0x7f), vec![0x7f]);
        assert_eq!(encode_len(0x80), vec![0x80, 0x01]);
        assert_eq!(encode_len(0xff), vec![0xff, 0x01]);
        assert_eq!(encode_len(0x100), vec![0x80, 0x02]);
        assert_eq!(encode_len(0x3fff), vec![0xff, 0x7f]);
    }

    #[test]
    fn base64_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    fn pk(byte: u8) -> Pubkey {
        Pubkey::new([byte; 32])
    }

    #[test]
    fn memo_message_layout() {
        let fee = pk(1);
        let msg = compile_message(&fee, [7u8; 32], &[memo_instruction("hi", &[fee])]);
        // one signer (the fee payer), and the memo program is a readonly non-signer.
        assert_eq!(msg.num_required_signatures, 1);
        // header
        assert_eq!(&msg.bytes[0..3], &[1, 0, 1]);
        // account count = 2 (fee payer + memo program)
        assert_eq!(msg.bytes[3], 2);
        // fee payer is the first account key
        assert_eq!(&msg.bytes[4..36], &fee.to_bytes());
        // blockhash sits right after the two 32-byte keys
        let bh_start = 4 + 32 * 2;
        assert_eq!(&msg.bytes[bh_start..bh_start + 32], &[7u8; 32]);
    }

    #[test]
    fn unsigned_tx_has_one_zero_sig_slot() {
        let fee = pk(1);
        let msg = compile_message(&fee, [0u8; 32], &[memo_instruction("x", &[fee])]);
        let b64 = unsigned_transaction_base64(&msg);
        // decode the shortvec + signature region by re-deriving: 1 sig => 65 byte prefix.
        // Just assert it is valid base64 of the expected total length.
        let raw_len = 1 + 64 + msg.bytes.len();
        assert_eq!(b64.len(), (raw_len + 2) / 3 * 4);
    }

    #[test]
    fn dedup_merges_writable_and_signer() {
        // The same key used as readonly in one ix and writable-signer in another
        // must collapse to one writable-signer account.
        let fee = pk(1);
        let key = pk(9);
        let sys = Pubkey::from_base58(crate::pubkey::known::SYSTEM_PROGRAM).unwrap();
        let ix_a = Instruction { program_id: sys, accounts: vec![AccountMeta::readonly(key)], data: vec![] };
        let ix_b = Instruction { program_id: sys, accounts: vec![AccountMeta::signer_writable(key)], data: vec![] };
        let msg = compile_message(&fee, [0u8; 32], &[ix_a, ix_b]);
        // fee payer + key(signer) => 2 signers; system program readonly non-signer.
        assert_eq!(msg.num_required_signatures, 2);
        assert_eq!(msg.bytes[0], 2); // num_required_signatures
        assert_eq!(msg.bytes[1], 0); // num_readonly_signed (key is writable signer)
        assert_eq!(msg.bytes[2], 1); // num_readonly_unsigned (system program)
    }

    #[test]
    fn durable_nonce_advance_shape() {
        let nonce = pk(2);
        let auth = pk(1);
        let ix = advance_nonce_instruction(&nonce, &auth);
        assert_eq!(ix.data, vec![4, 0, 0, 0]);
        assert_eq!(ix.accounts.len(), 3);
        assert!(ix.accounts[0].is_writable && !ix.accounts[0].is_signer); // nonce account
        assert!(ix.accounts[2].is_signer); // authority signs
    }
}
