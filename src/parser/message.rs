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
