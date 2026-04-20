use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, RgbaImage};
use qrcode::QrCode;
use std::fs;
use std::io::BufWriter;
use std::path::Path;

const MAX_FRAGMENT_SIZE: usize = 100;
const FRAME_DELAY_MS: u32 = 200;
const QR_MODULE_PX: u32 = 10;
const QR_QUIET_ZONE: u32 = 2;

fn main() {
    let bin_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_txs_bin");
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../test_txs_ur");

    if !bin_dir.exists() {
        eprintln!("No test_txs_bin directory found at {}", bin_dir.display());
        std::process::exit(1);
    }

    fs::create_dir_all(&out_dir).unwrap();

    let mut entries: Vec<_> = fs::read_dir(&bin_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "bin").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let path = entry.path();
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let data = fs::read(&path).unwrap();

        let tx_dir = out_dir.join(&name);
        fs::create_dir_all(&tx_dir).unwrap();

        let parts = generate_ur_parts(&data);
        write_txt(&tx_dir, &name, &parts);
        let frames = write_pngs(&tx_dir, &parts);
        write_gif(&tx_dir, &name, &frames);

        println!(
            "{}: {} bytes → {} frames",
            name,
            data.len(),
            parts.len()
        );
    }

    println!("\nDone. Output: {}", out_dir.display());
}

fn generate_ur_parts(data: &[u8]) -> Vec<String> {
    let mut encoder = ur::Encoder::bytes(data, MAX_FRAGMENT_SIZE).unwrap();
    let total = encoder.fragment_count();
    let mut parts = Vec::with_capacity(total);
    for _ in 0..total {
        parts.push(encoder.next_part().unwrap());
    }
    parts
}

fn write_txt(dir: &Path, name: &str, parts: &[String]) {
    let content = parts.join("\n");
    fs::write(dir.join(format!("{}.txt", name)), &content).unwrap();
}

fn write_pngs(dir: &Path, parts: &[String]) -> Vec<RgbaImage> {
    let mut images = Vec::with_capacity(parts.len());
    for (i, part) in parts.iter().enumerate() {
        let img = render_qr(part);
        let png_path = dir.join(format!("frame_{:03}.png", i + 1));
        img.save(&png_path).unwrap();
        images.push(img);
    }
    images
}

fn write_gif(dir: &Path, name: &str, frames: &[RgbaImage]) {
    let gif_path = dir.join(format!("{}.gif", name));
    let file = fs::File::create(&gif_path).unwrap();
    let writer = BufWriter::new(file);
    let mut encoder = GifEncoder::new_with_speed(writer, 10);
    encoder.set_repeat(Repeat::Infinite).unwrap();

    let delay = Delay::from_numer_denom_ms(FRAME_DELAY_MS, 1);
    for img in frames {
        let frame = Frame::from_parts(img.clone(), 0, 0, delay.clone());
        encoder.encode_frame(frame).unwrap();
    }
}

fn render_qr(data: &str) -> RgbaImage {
    let code = QrCode::new(data.as_bytes()).unwrap();
    let modules = code.width() as u32;
    let size = (modules + QR_QUIET_ZONE * 2) * QR_MODULE_PX;

    let mut img = RgbaImage::from_pixel(size, size, image::Rgba([255, 255, 255, 255]));

    for y in 0..modules {
        for x in 0..modules {
            use qrcode::Color;
            if code[(x as usize, y as usize)] == Color::Dark {
                let px = (x + QR_QUIET_ZONE) * QR_MODULE_PX;
                let py = (y + QR_QUIET_ZONE) * QR_MODULE_PX;
                for dy in 0..QR_MODULE_PX {
                    for dx in 0..QR_MODULE_PX {
                        img.put_pixel(px + dx, py + dy, image::Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }

    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn roundtrip_all_transactions() {
        let bin_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test_txs_bin");
        let ur_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test_txs_ur");

        for entry in fs::read_dir(&bin_dir).unwrap().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().map(|x| x == "bin").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let original = fs::read(&path).unwrap();

                let txt_path = ur_dir.join(&name).join(format!("{}.txt", name));
                let txt = fs::read_to_string(&txt_path).unwrap();
                let parts: Vec<&str> = txt.lines().collect();

                let mut decoder = ur::Decoder::default();
                for part in &parts {
                    decoder.receive(part).unwrap();
                }
                assert!(decoder.complete(), "{} did not complete", name);
                let decoded = decoder.message().unwrap().unwrap();
                assert_eq!(decoded, original, "{} roundtrip mismatch", name);
            }
        }
    }
}
