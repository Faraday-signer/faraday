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

pub(crate) fn read_disc8(data: &[u8], offset: usize) -> Result<[u8; 8], &'static str> {
    let end = offset.checked_add(8).ok_or("offset overflow")?;
    data.get(offset..end)
        .and_then(|s| s.try_into().ok())
        .ok_or("data truncated")
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
