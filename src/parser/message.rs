//! Solana transaction wire format deserializer.
//!
//! Handles both legacy and v0 (versioned) transactions.
//!
//! Legacy wire format:
//!   [num_signatures: u8]
//!   [signatures: 64 bytes each]
//!   [header: 3 bytes]
//!   [num_accounts: compact-u16] [accounts: 32 bytes each]
//!   [recent_blockhash: 32 bytes]
//!   [num_instructions: compact-u16] [instructions...]
//!
//! V0 wire format:
//!   [num_signatures: u8]
//!   [signatures: 64 bytes each]
//!   [0x80] (version prefix — high bit set, lower 7 bits = version number)
//!   [header: 3 bytes]
//!   [num_accounts: compact-u16] [accounts: 32 bytes each]
//!   [recent_blockhash: 32 bytes]
//!   [num_instructions: compact-u16] [instructions...]
//!   [num_address_table_lookups: compact-u16] [lookups...]

pub struct RawInstruction {
    pub program_id_index: usize,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

pub struct AddressTableLookup {
    pub account_key: [u8; 32],
    pub writable_indices: Vec<u8>,
    pub readonly_indices: Vec<u8>,
}

pub struct ParsedMessage {
    pub version: MessageVersion,
    pub num_required_signers: u8,
    pub accounts: Vec<[u8; 32]>,
    pub recent_blockhash: [u8; 32],
    pub instructions: Vec<RawInstruction>,
    /// Only populated for v0 transactions.
    pub address_table_lookups: Vec<AddressTableLookup>,
}

pub enum MessageVersion {
    Legacy,
    V0,
}

pub fn deserialize(tx_bytes: &[u8]) -> Result<ParsedMessage, &'static str> {
    let mut cur = Cursor::new(tx_bytes);

    // Skip signatures
    let num_sigs = cur.read_u8()? as usize;
    cur.skip(num_sigs * 64)?;

    // Detect version: if the first message byte has the high bit set, it's versioned.
    let version_byte = cur.peek()?;
    let version = if version_byte & 0x80 != 0 {
        cur.skip(1)?; // consume the version prefix byte
        match version_byte & 0x7f {
            0 => MessageVersion::V0,
            v => return Err(if v > 0 { "Unsupported transaction version" } else { unreachable!() }),
        }
    } else {
        MessageVersion::Legacy
    };

    // Message header (same layout for legacy and v0)
    let num_required_signers = cur.read_u8()?;
    let _num_readonly_signed = cur.read_u8()?;
    let _num_readonly_unsigned = cur.read_u8()?;

    // Static account keys
    let num_accounts = cur.read_compact_u16()?;
    let mut accounts = Vec::with_capacity(num_accounts);
    for _ in 0..num_accounts {
        accounts.push(cur.read_pubkey()?);
    }

    // Recent blockhash
    let recent_blockhash = cur.read_pubkey()?;

    // Instructions
    let num_instructions = cur.read_compact_u16()?;
    let mut instructions = Vec::with_capacity(num_instructions);
    for _ in 0..num_instructions {
        let program_id_index = cur.read_u8()? as usize;

        let num_ix_accounts = cur.read_compact_u16()?;
        let mut account_indices = Vec::with_capacity(num_ix_accounts);
        for _ in 0..num_ix_accounts {
            account_indices.push(cur.read_u8()?);
        }

        let data_len = cur.read_compact_u16()?;
        let data = cur.read_bytes(data_len)?.to_vec();

        instructions.push(RawInstruction { program_id_index, account_indices, data });
    }

    // Address lookup tables (v0 only)
    let mut address_table_lookups = Vec::new();
    if matches!(version, MessageVersion::V0) {
        let num_lookups = cur.read_compact_u16()?;
        for _ in 0..num_lookups {
            let account_key = cur.read_pubkey()?;

            let num_writable = cur.read_compact_u16()?;
            let mut writable_indices = Vec::with_capacity(num_writable);
            for _ in 0..num_writable {
                writable_indices.push(cur.read_u8()?);
            }

            let num_readonly = cur.read_compact_u16()?;
            let mut readonly_indices = Vec::with_capacity(num_readonly);
            for _ in 0..num_readonly {
                readonly_indices.push(cur.read_u8()?);
            }

            address_table_lookups.push(AddressTableLookup {
                account_key,
                writable_indices,
                readonly_indices,
            });
        }
    }

    Ok(ParsedMessage {
        version,
        num_required_signers,
        accounts,
        recent_blockhash,
        instructions,
        address_table_lookups,
    })
}

// === Cursor ===

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    fn peek(&self) -> Result<u8, &'static str> {
        self.data.get(self.pos).copied().ok_or("Unexpected end of data")
    }

    fn read_u8(&mut self) -> Result<u8, &'static str> {
        let b = self.data.get(self.pos).copied().ok_or("Unexpected end of data")?;
        self.pos += 1;
        Ok(b)
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], &'static str> {
        if self.pos + n > self.data.len() { return Err("Unexpected end of data"); }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    fn read_pubkey(&mut self) -> Result<[u8; 32], &'static str> {
        let bytes = self.read_bytes(32)?;
        let mut key = [0u8; 32];
        key.copy_from_slice(bytes);
        Ok(key)
    }

    fn skip(&mut self, n: usize) -> Result<(), &'static str> {
        if self.pos + n > self.data.len() { return Err("Unexpected end of data"); }
        self.pos += n;
        Ok(())
    }

    fn read_compact_u16(&mut self) -> Result<usize, &'static str> {
        let first = self.read_u8()? as usize;
        if first < 0x80 { return Ok(first); }
        let second = self.read_u8()? as usize;
        if second < 0x80 { return Ok((first & 0x7f) | (second << 7)); }
        let third = self.read_u8()? as usize;
        Ok((first & 0x7f) | ((second & 0x7f) << 7) | (third << 14))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal legacy transaction: 1 sig, 2 accounts, 1 System Transfer instruction.
    fn legacy_transfer_tx(lamports: u64) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1u8);                          // num_signatures
        tx.extend_from_slice(&[0u8; 64]);      // signature placeholder

        // message header
        tx.push(1); // num_required_signers
        tx.push(0); // num_readonly_signed
        tx.push(1); // num_readonly_unsigned

        // accounts (compact-u16 = 2)
        tx.push(2);
        tx.extend_from_slice(&[0x01u8; 32]);   // signer
        tx.extend_from_slice(&[0x00u8; 32]);   // system program

        tx.extend_from_slice(&[0xABu8; 32]);   // recent blockhash

        // 1 instruction
        tx.push(1);
        tx.push(1);  // program_id_index = 1
        tx.push(1);  // 1 account index
        tx.push(0);  // account_indices[0] = 0
        tx.push(12); // data_len = 12 (compact-u16)
        tx.extend_from_slice(&[2u8, 0, 0, 0]); // Transfer type
        tx.extend_from_slice(&lamports.to_le_bytes());
        tx
    }

    /// Same as above but with 0x80 version prefix (v0, no lookup tables).
    fn v0_transfer_tx(lamports: u64) -> Vec<u8> {
        let mut tx = Vec::new();
        tx.push(1u8);
        tx.extend_from_slice(&[0u8; 64]);

        tx.push(0x80); // version prefix: v0

        tx.push(1);
        tx.push(0);
        tx.push(1);

        tx.push(2);
        tx.extend_from_slice(&[0x01u8; 32]);
        tx.extend_from_slice(&[0x00u8; 32]);

        tx.extend_from_slice(&[0xABu8; 32]);

        tx.push(1);
        tx.push(1);
        tx.push(1);
        tx.push(0);
        tx.push(12);
        tx.extend_from_slice(&[2u8, 0, 0, 0]);
        tx.extend_from_slice(&lamports.to_le_bytes());

        tx.push(0); // num_address_table_lookups = 0
        tx
    }

    #[test]
    fn test_legacy_deserialize() {
        let tx = legacy_transfer_tx(1_000_000_000);
        let msg = deserialize(&tx).unwrap();
        assert!(matches!(msg.version, MessageVersion::Legacy));
        assert_eq!(msg.num_required_signers, 1);
        assert_eq!(msg.accounts.len(), 2);
        assert_eq!(msg.accounts[0], [0x01u8; 32]);
        assert_eq!(msg.accounts[1], [0x00u8; 32]);
        assert_eq!(msg.recent_blockhash, [0xABu8; 32]);
        assert_eq!(msg.instructions.len(), 1);
        assert_eq!(msg.address_table_lookups.len(), 0);
    }

    #[test]
    fn test_legacy_instruction_fields() {
        let tx = legacy_transfer_tx(500_000_000);
        let msg = deserialize(&tx).unwrap();
        let ix = &msg.instructions[0];
        assert_eq!(ix.program_id_index, 1);
        assert_eq!(ix.account_indices, vec![0]);
        assert_eq!(&ix.data[..4], &[2, 0, 0, 0]); // Transfer discriminant
        let lamports = u64::from_le_bytes(ix.data[4..12].try_into().unwrap());
        assert_eq!(lamports, 500_000_000);
    }

    #[test]
    fn test_v0_detected() {
        let tx = v0_transfer_tx(1_000_000_000);
        let msg = deserialize(&tx).unwrap();
        assert!(matches!(msg.version, MessageVersion::V0));
        assert_eq!(msg.accounts.len(), 2);
        assert_eq!(msg.instructions.len(), 1);
        assert_eq!(msg.address_table_lookups.len(), 0);
    }

    #[test]
    fn test_empty_transaction_errors() {
        assert!(deserialize(&[]).is_err());
    }

    #[test]
    fn test_truncated_transaction_errors() {
        // Only num_sigs byte, no signatures
        assert!(deserialize(&[1u8]).is_err());
    }

    #[test]
    fn test_compact_u16_multibyte() {
        // Value 128 encodes as [0x80, 0x01]
        // Build a tx where num_accounts = 128 (would be too short to fully parse,
        // but we can verify the compact-u16 reading fails gracefully)
        let mut tx = Vec::new();
        tx.push(0u8);            // 0 signatures
        tx.push(1); tx.push(0); tx.push(0); // header
        tx.push(0x80); tx.push(0x01); // compact-u16: 128 accounts — truncated intentionally
        assert!(deserialize(&tx).is_err());
    }
}
