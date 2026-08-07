//! Build script: downloads BIP39 wordlist from bitcoin/bips and verifies SHA256.
//!
//! Source: https://github.com/bitcoin/bips/blob/master/bip-0039/english.txt
//! Expected SHA256: 2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda

use std::fs;
use std::path::Path;
use std::process::Command;

const WORDLIST_URL: &str =
    "https://raw.githubusercontent.com/bitcoin/bips/master/bip-0039/english.txt";
const EXPECTED_SHA256: &str =
    "2f5eed53a4727b4bf8880d8f3f199efc90e58503646d9ff8eff3a2ed3b24dbda";

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let wordlist_path = Path::new(&out_dir).join("bip39_english.txt");

    // Only download if not already cached
    if !wordlist_path.exists() {
        println!("cargo:warning=Downloading BIP39 wordlist from bitcoin/bips...");

        let output = Command::new("curl")
            .args(["-sL", "--fail", WORDLIST_URL])
            .output()
            .expect("Failed to run curl — is it installed?");

        if !output.status.success() {
            panic!(
                "Failed to download BIP39 wordlist from {}\nHTTP error: {}",
                WORDLIST_URL,
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let data = output.stdout;

        // Verify SHA256
        let hash = sha256_bytes(&data);
        if hash != EXPECTED_SHA256 {
            panic!(
                "BIP39 wordlist SHA256 mismatch!\n  Expected: {}\n  Got:      {}\n  Source: {}",
                EXPECTED_SHA256, hash, WORDLIST_URL
            );
        }

        // Verify word count
        let text = String::from_utf8_lossy(&data);
        let count = text.lines().filter(|l| !l.is_empty()).count();
        if count != 2048 {
            panic!("BIP39 wordlist has {} words, expected 2048", count);
        }

        fs::write(&wordlist_path, &data).expect("Failed to write wordlist");
        println!("cargo:warning=BIP39 wordlist verified (SHA256 OK, 2048 words)");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn sha256_bytes(data: &[u8]) -> String {
    use std::fmt::Write;

    // Minimal SHA256 for the build script (no external deps in build.rs)
    let hash = {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize()
    };

    let mut hex = String::with_capacity(64);
    for byte in hash {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }
    hex
}

// Minimal SHA256 implementation for build script (no dependencies)
struct Sha256 {
    state: [u32; 8],
    buffer: Vec<u8>,
    total_len: u64,
}

impl Sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    fn new() -> Self {
        Sha256 {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
                0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
            ],
            buffer: Vec::new(),
            total_len: 0,
        }
    }

    fn update(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
        self.total_len += data.len() as u64;

        while self.buffer.len() >= 64 {
            let block: Vec<u8> = self.buffer.drain(..64).collect();
            self.process_block(&block);
        }
    }

    fn process_block(&mut self, block: &[u8]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([block[i*4], block[i*4+1], block[i*4+2], block[i*4+3]]);
        }
        for i in 16..64 {
            let s0 = w[i-15].rotate_right(7) ^ w[i-15].rotate_right(18) ^ (w[i-15] >> 3);
            let s1 = w[i-2].rotate_right(17) ^ w[i-2].rotate_right(19) ^ (w[i-2] >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = h.wrapping_add(s1).wrapping_add(ch).wrapping_add(Self::K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);

            h = g; g = f; f = e; e = d.wrapping_add(t1);
            d = c; c = b; b = a; a = t1.wrapping_add(t2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }

    fn finalize(mut self) -> [u8; 32] {
        let bit_len = self.total_len * 8;
        self.buffer.push(0x80);
        while self.buffer.len() % 64 != 56 {
            self.buffer.push(0);
        }
        self.buffer.extend_from_slice(&bit_len.to_be_bytes());

        while self.buffer.len() >= 64 {
            let block: Vec<u8> = self.buffer.drain(..64).collect();
            self.process_block(&block);
        }

        let mut result = [0u8; 32];
        for (i, &val) in self.state.iter().enumerate() {
            result[i*4..i*4+4].copy_from_slice(&val.to_be_bytes());
        }
        result
    }
}
