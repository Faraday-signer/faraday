//! Bounds-safe readers for the instruction-data byte slices the parser walks.
//!
//! Every reader validates `offset..offset + N` against `data.len()` via
//! `slice::get`, so the compiler enforces the invariant that used to live in
//! pairs of `if data.len() < N { ... }` / `.try_into().unwrap()` lines. On a
//! truncated input the returned `Err` flows up through `?` to the instruction
//! parser's `match decode(...)`, which renders it as a `ReviewItem::Warning`
//! on the review screen instead of panicking the device.

pub(crate) fn read_u64_le(data: &[u8], offset: usize) -> Result<u64, &'static str> {
    let end = offset.checked_add(8).ok_or("offset overflow")?;
    data.get(offset..end)
        .and_then(|s| s.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or("data truncated")
}

pub(crate) fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, &'static str> {
    let end = offset.checked_add(4).ok_or("offset overflow")?;
    data.get(offset..end)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or("data truncated")
}

pub(crate) fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, &'static str> {
    let end = offset.checked_add(2).ok_or("offset overflow")?;
    data.get(offset..end)
        .and_then(|s| s.try_into().ok())
        .map(u16::from_le_bytes)
        .ok_or("data truncated")
}

pub(crate) fn read_disc8(data: &[u8], offset: usize) -> Result<[u8; 8], &'static str> {
    let end = offset.checked_add(8).ok_or("offset overflow")?;
    data.get(offset..end)
        .and_then(|s| s.try_into().ok())
        .ok_or("data truncated")
}

/// Decodes the 19-byte trailer that aggregator swap-instruction data carries
/// after a variable-length route plan: `in_amount(u64) | out_amount(u64) |
/// slippage_bps(u16) | platform_fee_bps(u8)`. Used by Jupiter's RoutePlanFirst
/// layouts and by DFlow.
pub(crate) fn read_swap_footer(data: &[u8]) -> Result<(u64, u64, u16, u8), &'static str> {
    const FOOTER: usize = 8 + 8 + 2 + 1;
    if data.len() < 8 + FOOTER {
        return Err("data too short for trailing amounts");
    }
    let pos = data.len() - FOOTER;
    let in_amount = read_u64_le(data, pos)?;
    let out_amount = read_u64_le(data, pos + 8)?;
    let lo = *data.get(pos + 16).ok_or("data truncated")?;
    let hi = *data.get(pos + 17).ok_or("data truncated")?;
    let slippage_bps = u16::from_le_bytes([lo, hi]);
    let fee_bps = *data.get(pos + 18).ok_or("data truncated")?;
    Ok((in_amount, out_amount, slippage_bps, fee_bps))
}

/// Truncated `<head>..<tail>` rendering of a 32-byte pubkey for review-line
/// display where the full base58 form would overflow the column.
pub(crate) fn pubkey_short(key: &[u8; 32]) -> String {
    let b58 = bs58::encode(key).into_string();
    if b58.len() >= 8 {
        format!("{}..{}", &b58[..4], &b58[b58.len() - 4..])
    } else {
        b58
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_le_reads_at_offset() {
        let data = [0u8, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(read_u64_le(&data, 0).unwrap(), 0x0000_0001_0000_0000);
        assert_eq!(read_u64_le(&data, 8).unwrap(), 2);
    }

    #[test]
    fn u64_le_rejects_truncated() {
        assert!(read_u64_le(&[1, 2, 3], 0).is_err());
        assert!(read_u64_le(&[0u8; 8], 1).is_err());
    }

    #[test]
    fn u64_le_rejects_offset_overflow() {
        assert!(read_u64_le(&[0u8; 8], usize::MAX - 3).is_err());
    }

    #[test]
    fn u32_le_reads_at_offset() {
        let data = [1u8, 0, 0, 0, 5, 0, 0, 0];
        assert_eq!(read_u32_le(&data, 0).unwrap(), 1);
        assert_eq!(read_u32_le(&data, 4).unwrap(), 5);
    }

    #[test]
    fn disc8_rejects_truncated() {
        assert!(read_disc8(&[0u8; 7], 0).is_err());
        assert!(read_disc8(&[0u8; 8], 1).is_err());
    }

    #[test]
    fn disc8_reads_at_offset() {
        let data = [0u8; 16];
        assert_eq!(read_disc8(&data, 0).unwrap(), [0u8; 8]);
        assert_eq!(read_disc8(&data, 8).unwrap(), [0u8; 8]);
    }
}
