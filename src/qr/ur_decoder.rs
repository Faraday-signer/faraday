//! Accumulates UR (Uniform Resource) fountain-coded QR frames and
//! reconstructs the original payload once enough frames are received.

extern crate alloc;

/// Wraps `ur::Decoder` with progress tracking and UR-detection logic.
pub struct UrAccumulator {
    decoder: ur::Decoder,
    started: bool,
    /// Set of fragment sequence numbers the decoder has accepted. Tracking
    /// unique received parts (rather than just the most recent) is what the
    /// scan screen needs: a "last seen" display makes duplicates look like
    /// progress and hides missing parts. The inner `ur::Decoder` doesn't
    /// expose this state.
    received_seqs: alloc::collections::BTreeSet<usize>,
    /// Total fragment count parsed from any received fragment's header.
    /// All fragments of the same message share the same total, so whichever
    /// part arrives first sets it.
    total: Option<usize>,
}

impl UrAccumulator {
    pub fn new() -> Self {
        Self {
            decoder: ur::Decoder::default(),
            started: false,
            received_seqs: alloc::collections::BTreeSet::new(),
            total: None,
        }
    }

    /// Returns true if the string looks like a UR fragment (starts with "ur:").
    pub fn is_ur(data: &str) -> bool {
        data.starts_with("ur:")
    }

    /// Feed a UR fragment. Returns Ok(true) when the message is complete.
    ///
    /// Two subtleties this handles:
    ///
    /// - **Redundancy parts have `seq > total`.** UR fountain streams send
    ///   pure parts numbered `1..=total`, then combined parts numbered
    ///   `total+1..`. Combined parts advance the decoder but aren't
    ///   progress the user cares about — counting them produced the
    ///   `22/21` display the scan-screen diagnostic showed in practice.
    /// - **Stream change.** If the user points the camera at a *different*
    ///   UR animation mid-scan, the inner decoder rejects the new part
    ///   (checksum mismatch). Rather than leaving stale state that
    ///   merges with the new stream, we reset on rejection and re-feed
    ///   the part into a fresh decoder — so the next scan starts clean.
    pub fn receive(&mut self, part: &str) -> Result<bool, UrError> {
        let parsed = parse_seq_total(part);
        let accept = |this: &mut Self| {
            this.started = true;
            if let Some((seq, total)) = parsed {
                if seq <= total {
                    this.received_seqs.insert(seq);
                }
                this.total = Some(total);
            }
            Ok(this.decoder.complete())
        };

        match self.decoder.receive(part) {
            Ok(()) => accept(self),
            Err(_) => {
                // Stream mismatch. Wipe state and try the part fresh —
                // it's almost always the first part of a new stream.
                self.decoder = ur::Decoder::default();
                self.received_seqs.clear();
                self.total = None;
                self.started = false;
                match self.decoder.receive(part) {
                    Ok(()) => accept(self),
                    Err(_) => Err(UrError::InvalidPart),
                }
            }
        }
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

    /// Live scan progress: `(received, total)` — number of distinct
    /// fragment seq values accepted by the decoder, and the total count
    /// from the UR header. `None` until the first fragment arrives.
    pub fn progress(&self) -> Option<(usize, usize)> {
        self.total.map(|t| (self.received_seqs.len(), t))
    }

    /// Reset the accumulator to start a fresh decode session.
    pub fn reset(&mut self) {
        self.decoder = ur::Decoder::default();
        self.started = false;
        self.received_seqs.clear();
        self.total = None;
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
        assert!(acc.progress().is_none());
    }

    #[test]
    fn progress_counts_unique_received_fragments() {
        let data = vec![0xAB; 500];
        let mut encoder = ur::Encoder::bytes(&data, 100).unwrap();
        let total = encoder.fragment_count();
        let mut acc = UrAccumulator::new();
        assert_eq!(acc.progress(), None);
        for i in 1..=total {
            let part = encoder.next_part().unwrap();
            acc.receive(&part).unwrap();
            let (received, tot) = acc.progress().expect("progress after first part");
            assert_eq!(received, i, "each pure fragment should bump received by one");
            assert_eq!(tot, total);
        }
    }

    #[test]
    fn progress_caps_at_total_when_fountain_sends_redundancy_parts() {
        // A fountain encoder produces pure parts (seq 1..=total) then
        // redundancy parts (seq > total). Our progress display must not
        // show something like `22/21` — redundancy parts help the decoder
        // but aren't user-visible progress.
        let data = vec![0xEF; 800];
        let mut encoder = ur::Encoder::bytes(&data, 50).unwrap();
        let total = encoder.fragment_count();
        let mut acc = UrAccumulator::new();
        // Drive enough parts through that we pass pure-parts count and
        // into the redundancy range.
        for _ in 0..(total * 3) {
            let part = encoder.next_part().unwrap();
            acc.receive(&part).unwrap();
            if let Some((received, tot)) = acc.progress() {
                assert!(received <= tot, "received {} > total {}", received, tot);
            }
        }
    }

    #[test]
    fn switching_streams_mid_scan_resets_progress() {
        // User points camera at stream A, then switches to stream B.
        // Stream B's parts must not appear as progress on top of A's —
        // the display should restart from stream B's first accepted part.
        let data_a = vec![0xAA; 300];
        let data_b = vec![0xBB; 400];
        let mut enc_a = ur::Encoder::bytes(&data_a, 50).unwrap();
        let mut enc_b = ur::Encoder::bytes(&data_b, 40).unwrap();
        let mut acc = UrAccumulator::new();

        // Feed two parts from A.
        for _ in 0..2 {
            acc.receive(&enc_a.next_part().unwrap()).unwrap();
        }
        let (a_count, _) = acc.progress().expect("after A parts");
        assert_eq!(a_count, 2);

        // Switch to B. First B part should trigger reset + clean start.
        acc.receive(&enc_b.next_part().unwrap()).unwrap();
        let (b_count, b_total) = acc.progress().expect("after first B part");
        assert_eq!(b_count, 1);
        assert_eq!(b_total, enc_b.fragment_count());
    }

    #[test]
    fn progress_is_stable_across_duplicate_receives() {
        // A slow decoder will read the same GIF frame twice before the
        // animation advances; the progress counter must not inflate from
        // redundant receives.
        let data = vec![0xCD; 500];
        let mut encoder = ur::Encoder::bytes(&data, 100).unwrap();
        let part1 = encoder.next_part().unwrap();
        let mut acc = UrAccumulator::new();
        acc.receive(&part1).unwrap();
        let first = acc.progress().unwrap();
        acc.receive(&part1).unwrap();
        let second = acc.progress().unwrap();
        assert_eq!(first, second);
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

    #[test]
    fn accepts_bc_ur_js_fixture_frames() {
        // Compatibility fixture generated from @ngraveio/bc-ur:
        // - payload: bytes[i] = (i*29 + 11) & 0xff for i in 0..500
        // - fragment size: 100
        // - emitted with encodeWhole() (pure parts 1..=5)
        // - IMPORTANT: UR constructed as `new UR(rawBytes, "bytes")`
        //   (not UR.fromBuffer, which wraps payload in CBOR bytes)
        let frames = [
            "ur:bytes/1-5/lpadahcfadwkcylulowplphdiebddefeidlbnsrhtbwfbedpgeiolroyrnuyyabzeygwjzldolsrvtzccyemghjsmnpyspvwaoctfnhkkomupfsnwdatdkfphykgmkretdwsbndtfgialantrdtswkbydmgrislpoersuoytcmeogdjnleosssvyzecwetgojpmypssovaaxcxfshtktmwpatowmaydafwwyotfskt",
            "ur:bytes/2-5/lpaoahcfadwkcylulowplphdiehekenlrptewtbtdrflielynnrktpykbgdlgsinlnotrtutzscheegyjtlupdskvozmceeshfjkmhpmsgvdaaclfmhpksmdprtkwpasdsfxhnkinyrltywnbadnfdihlfnerftaynbwdygtimltoxseuezocsecgmjllkptswvlaecafthgjymeplsbvsahcpfhhhkkmtcsfgbnyn",
            "ur:bytes/3-5/lpaxahcfadwkcylulowplphdieqdtiwebkdifyhskbndrotlwzbsdwgaiylsnbrytnylbbehgljeloonsaurztcfengujolgpkstveadckfrhdkpmopesfwlamcnfzhlknmsqzttwybddefeidlbnsrhtbwfbedpgeiolroyrnuyyabzeygwjzldolsrvtzccyemghjsmnpyspvwaoctfnhkkomupfsnwdtktyoynl",
            "ur:bytes/4-5/lpaaahcfadwkcylulowplphdieatdkfphykgmkretdwsbndtfgialantrdtswkbydmgrislpoersuoytcmeogdjnleosssvyzecwetgojpmypssovaaxcxfshtktmwpatowmaydafwhekenlrptewtbtdrflielynnrktpykbgdlgsinlnotrtutzscheegyjtlupdskvozmceeshfjkmhpmsgvdaaclfmcfgmztga",
            "ur:bytes/5-5/lpahahcfadwkcylulowplphdiehpksmdprtkwpasdsfxhnkinyrltywnbadnfdihlfnerftaynbwdygtimltoxseuezocsecgmjllkptswvlaecafthgjymeplsbvsahcpfhhhkkmtqdtiwebkdifyhskbndrotlwzbsdwgaiylsnbrytnylbbehgljeloonsaurztcfengujolgpkstveadckfrhdkpmofxkkosjs",
        ];

        let expected: Vec<u8> = (0..500)
            .map(|i| ((i * 29 + 11) & 0xff) as u8)
            .collect();

        let mut acc = UrAccumulator::new();
        for frame in frames {
            acc.receive(frame).expect("valid fixture frame");
        }
        assert!(acc.complete());
        assert_eq!(acc.message().unwrap(), expected);
    }
}
