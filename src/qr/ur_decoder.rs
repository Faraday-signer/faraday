//! Accumulates UR (Uniform Resource) fountain-coded QR frames and
//! reconstructs the original payload once enough frames are received.

/// Wraps `ur::Decoder` with progress tracking and UR-detection logic.
pub struct UrAccumulator {
    decoder: ur::Decoder,
    started: bool,
    /// `(seq, total)` parsed from the most recent fragment's header. Used by
    /// the scan screen to surface live progress — the inner `ur::Decoder`
    /// does not expose which parts it has received.
    last_part: Option<(usize, usize)>,
}

impl UrAccumulator {
    pub fn new() -> Self {
        Self {
            decoder: ur::Decoder::default(),
            started: false,
            last_part: None,
        }
    }

    /// Returns true if the string looks like a UR fragment (starts with "ur:").
    pub fn is_ur(data: &str) -> bool {
        data.starts_with("ur:")
    }

    /// Feed a UR fragment. Returns Ok(true) when the message is complete.
    pub fn receive(&mut self, part: &str) -> Result<bool, UrError> {
        // Record (seq, total) from the URI before handing to the decoder so
        // even a decoder-rejected part tells the user the camera is reading.
        self.last_part = parse_seq_total(part);
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

    /// Most recent `(seq, total)` observed from a UR fragment header. Used
    /// by the scan screen as a live progress readout.
    pub fn last_part(&self) -> Option<(usize, usize)> {
        self.last_part
    }

    /// Reset the accumulator to start a fresh decode session.
    pub fn reset(&mut self) {
        self.decoder = ur::Decoder::default();
        self.started = false;
        self.last_part = None;
    }
}

/// Extract `(seq, total)` from a UR fragment URI: `ur:<type>/<seq>-<total>/<data>`.
/// Returns None for single-part URIs (`ur:<type>/<data>`) or anything malformed.
fn parse_seq_total(part: &str) -> Option<(usize, usize)> {
    let body = part.strip_prefix("ur:")?;
    let after_type = body.split_once('/')?.1;
    let seq_total = after_type.split_once('/')?.0;
    let (seq, total) = seq_total.split_once('-')?;
    Some((seq.parse().ok()?, total.parse().ok()?))
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
        assert!(acc.last_part().is_none());
    }

    #[test]
    fn last_part_tracks_multipart_progress() {
        let data = vec![0xAB; 500];
        let mut encoder = ur::Encoder::bytes(&data, 100).unwrap();
        let total = encoder.fragment_count();
        let mut acc = UrAccumulator::new();
        assert_eq!(acc.last_part(), None);
        for i in 1..=total {
            let part = encoder.next_part().unwrap();
            acc.receive(&part).unwrap();
            let (seq, tot) = acc.last_part().expect("seq-total parsed");
            assert_eq!(seq, i);
            assert_eq!(tot, total);
        }
    }

    #[test]
    fn parse_seq_total_handles_well_formed() {
        assert_eq!(parse_seq_total("ur:bytes/3-7/lpaxat..."), Some((3, 7)));
        assert_eq!(parse_seq_total("ur:bytes/1-1/data"), Some((1, 1)));
    }

    #[test]
    fn parse_seq_total_rejects_malformed() {
        assert_eq!(parse_seq_total("ur:bytes/data"), None);
        assert_eq!(parse_seq_total("not:a:ur"), None);
        assert_eq!(parse_seq_total("ur:bytes/abc-def/data"), None);
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
