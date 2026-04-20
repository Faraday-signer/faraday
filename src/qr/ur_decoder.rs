//! Accumulates UR (Uniform Resource) fountain-coded QR frames and
//! reconstructs the original payload once enough frames are received.

/// Wraps `ur::Decoder` with progress tracking and UR-detection logic.
pub struct UrAccumulator {
    decoder: ur::Decoder,
    started: bool,
}

impl UrAccumulator {
    pub fn new() -> Self {
        Self {
            decoder: ur::Decoder::default(),
            started: false,
        }
    }

    /// Returns true if the string looks like a UR fragment (starts with "ur:").
    pub fn is_ur(data: &str) -> bool {
        data.starts_with("ur:")
    }

    /// Feed a UR fragment. Returns Ok(true) when the message is complete.
    pub fn receive(&mut self, part: &str) -> Result<bool, UrError> {
        self.decoder.receive(part).map_err(|_| UrError::InvalidPart)?;
        self.started = true;
        Ok(self.decoder.complete())
    }

    /// Extract the reconstructed message. Only valid after `receive` returns Ok(true).
    pub fn message(&self) -> Option<Vec<u8>> {
        self.decoder.message().ok().flatten()
    }

    /// Whether at least one frame has been received.
    pub fn started(&self) -> bool {
        self.started
    }

    /// Returns true when enough frames have been accumulated.
    pub fn complete(&self) -> bool {
        self.decoder.complete()
    }

    /// Reset the accumulator to start a fresh decode session.
    pub fn reset(&mut self) {
        self.decoder = ur::Decoder::default();
        self.started = false;
    }
}

#[derive(Debug)]
pub enum UrError {
    InvalidPart,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_part_message() {
        let data = b"hello world";
        let mut encoder = ur::Encoder::bytes(data, 200).unwrap();
        let part = encoder.next_part().unwrap();

        assert!(UrAccumulator::is_ur(&part));
        let mut acc = UrAccumulator::new();
        assert!(!acc.started());
        let done = acc.receive(&part).unwrap();
        assert!(done);
        assert!(acc.started());
        assert_eq!(acc.message().unwrap(), data);
    }

    #[test]
    fn multi_part_message() {
        let data = vec![0xAB; 500];
        let mut encoder = ur::Encoder::bytes(&data, 100).unwrap();
        let total = encoder.fragment_count();

        let mut acc = UrAccumulator::new();
        for _ in 0..total {
            let part = encoder.next_part().unwrap();
            let done = acc.receive(&part).unwrap();
            if done {
                break;
            }
        }
        assert!(acc.complete());
        assert_eq!(acc.message().unwrap(), data);
    }

    #[test]
    fn reset_clears_state() {
        let data = b"test";
        let mut encoder = ur::Encoder::bytes(data, 200).unwrap();
        let part = encoder.next_part().unwrap();

        let mut acc = UrAccumulator::new();
        acc.receive(&part).unwrap();
        assert!(acc.complete());

        acc.reset();
        assert!(!acc.started());
        assert!(!acc.complete());
    }

    #[test]
    fn is_ur_detection() {
        assert!(UrAccumulator::is_ur("ur:bytes/1-3/lpadao..."));
        assert!(!UrAccumulator::is_ur("not a ur string"));
        assert!(!UrAccumulator::is_ur(""));
    }

    #[test]
    fn roundtrip_real_transaction() {
        let bin_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("testdata/test_txs_bin");
        if !bin_dir.exists() {
            return;
        }
        let entries: Vec<_> = std::fs::read_dir(&bin_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "bin").unwrap_or(false))
            .collect();
        for entry in entries {
            let original = std::fs::read(entry.path()).unwrap();
            let mut encoder = ur::Encoder::bytes(&original, 100).unwrap();
            let total = encoder.fragment_count();

            let mut acc = UrAccumulator::new();
            for _ in 0..total {
                let part = encoder.next_part().unwrap();
                if acc.receive(&part).unwrap() {
                    break;
                }
            }
            assert!(acc.complete());
            assert_eq!(acc.message().unwrap(), original);
        }
    }
}
