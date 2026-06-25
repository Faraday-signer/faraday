//! ST7789T3 240x320 display driver via ESP-IDF SPI.
//!
//! Pin mapping for Waveshare ESP32-S3-Touch-LCD-2 (confirmed against the
//! board schematic and CircuitPython board definition):
//!   MOSI = GPIO 38  (LCD0MOSI)
//!   SCLK = GPIO 39  (LCD0SCLK)
//!   CS   = GPIO 45  (LCD0CS)
//!   DC   = GPIO 42  (LCD0DC) — also JTAG MTMS; see gpio_reset_pin() in main.rs
//!   RST  = GPIO 0   (LCD0RST) — shared with BOOT strap; not driven, we use
//!                    software reset (0x01) instead to avoid disturbing boot.
//!   BL   = GPIO 1   (LCD0BL)
//!
//! CS is managed manually (SpiDeviceDriver created with no hardware CS) so a
//! command and its data/pixels stay within a single CS assertion.

use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{raw::RawData, Rgb565},
    Pixel,
};
use esp32_common::BoardDisplay;
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Output, PinDriver};
use esp_idf_hal::ledc::LedcDriver;
use esp_idf_hal::spi::{SpiDeviceDriver, SpiDriver};

pub const WIDTH: u32 = 240;
pub const HEIGHT: u32 = 320;

/// Per-SPI-transaction chunk size. Matches the bus DMA limit (just under the
/// 32 767-byte ESP32-S3 cap, 4-byte aligned).
const FLUSH_CHUNK: usize = 32764;

pub struct Display<'d> {
    spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    cs: PinDriver<'d, Output>,
    dc: PinDriver<'d, Output>,
    bl: LedcDriver<'d>,
    buffer: Vec<u8>,
    // Internal, DMA-capable staging buffer for flush. The frame buffer lives in
    // PSRAM (it's far larger than the SPIRAM-internal malloc threshold), and
    // SPI-DMA-ing straight from PSRAM underruns the SPI FIFO whenever the camera
    // is saturating the PSRAM bus — which paints random black blocks and stalls
    // the flush. Copying each chunk into internal RAM first means the SPI engine
    // only ever DMAs from fast, uncontended memory.
    staging: *mut u8,
}

impl<'d> Display<'d> {
    pub fn new(
        spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
        cs: PinDriver<'d, Output>,
        dc: PinDriver<'d, Output>,
        bl: LedcDriver<'d>,
    ) -> Self {
        let buffer = vec![0u8; (WIDTH * HEIGHT * 2) as usize];

        // Allocate the flush staging buffer in internal, DMA-capable RAM.
        let staging = unsafe {
            esp_idf_sys::heap_caps_malloc(
                FLUSH_CHUNK,
                (esp_idf_sys::MALLOC_CAP_DMA | esp_idf_sys::MALLOC_CAP_INTERNAL) as u32,
            ) as *mut u8
        };
        assert!(!staging.is_null(), "failed to allocate internal DMA staging buffer");

        let mut display = Self { spi, cs, dc, bl, buffer, staging };
        display.init();
        display.set_backlight(30);

        display
    }

    fn command(&mut self, cmd: u8) {
        self.dc.set_low().unwrap_or_else(|e| log::error!("dc low: {e:?}"));
        self.cs.set_low().unwrap_or_else(|e| log::error!("cs low: {e:?}"));
        self.spi.write(&[cmd]).unwrap_or_else(|e| log::error!("spi cmd 0x{cmd:02X}: {e:?}"));
        self.cs.set_high().unwrap_or_else(|e| log::error!("cs high: {e:?}"));
    }

    fn data_byte(&mut self, val: u8) {
        self.dc.set_high().unwrap_or_else(|e| log::error!("dc high: {e:?}"));
        self.cs.set_low().unwrap_or_else(|e| log::error!("cs low: {e:?}"));
        self.spi.write(&[val]).unwrap_or_else(|e| log::error!("spi data 0x{val:02X}: {e:?}"));
        self.cs.set_high().unwrap_or_else(|e| log::error!("cs high: {e:?}"));
    }

    fn data(&mut self, vals: &[u8]) {
        self.dc.set_high().unwrap_or_else(|e| log::error!("dc high: {e:?}"));
        self.cs.set_low().unwrap_or_else(|e| log::error!("cs low: {e:?}"));
        self.spi.write(vals).unwrap_or_else(|e| log::error!("spi data[{}]: {e:?}", vals.len()));
        self.cs.set_high().unwrap_or_else(|e| log::error!("cs high: {e:?}"));
    }

    fn init(&mut self) {
        // Software reset (no hardware RST line driven — see module docs).
        self.command(0x01);
        FreeRtos::delay_ms(150);

        // Sleep out
        self.command(0x11);
        FreeRtos::delay_ms(500);

        // COLMOD: 16-bit RGB565
        self.command(0x3A);
        self.data_byte(0x55);
        FreeRtos::delay_ms(10);

        // RAMCTRL: interface/data format
        self.command(0xB0);
        self.data(&[0x00, 0xF0]);
        FreeRtos::delay_ms(10);

        // Color inversion ON (required for IPS panels)
        self.command(0x21);
        FreeRtos::delay_ms(10);

        // Normal display mode ON
        self.command(0x13);
        FreeRtos::delay_ms(10);

        // MADCTL: 0x00 = portrait 240x320, top-left origin
        self.command(0x36);
        self.data_byte(0x00);

        // Display ON
        self.command(0x29);
        FreeRtos::delay_ms(500);
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        self.command(0x2A);
        self.data(&[
            (x0 >> 8) as u8, (x0 & 0xFF) as u8,
            ((x1 - 1) >> 8) as u8, ((x1 - 1) & 0xFF) as u8,
        ]);

        self.command(0x2B);
        self.data(&[
            (y0 >> 8) as u8, (y0 & 0xFF) as u8,
            ((y1 - 1) >> 8) as u8, ((y1 - 1) & 0xFF) as u8,
        ]);
    }

    pub fn flush(&mut self) {
        self.set_window(0, 0, WIDTH as u16, HEIGHT as u16);
        // RAMWR + pixel data within a single CS assertion.
        // ESP32-S3 SPI hardware limits each transaction to 32 767 bytes
        // (18-bit bit-length register), so the frame buffer (153 600 bytes)
        // must be chunked. Each chunk is copied from the PSRAM frame buffer
        // into the internal DMA staging buffer first, so the SPI engine never
        // DMAs from PSRAM (see `staging`). With DMA enabled, each write yields
        // to FreeRTOS via a semaphore — the watchdog idle task gets to run
        // between chunks. 32 764 bytes = just under the limit, 5 chunks total.
        let _ = self.dc.set_low();
        let _ = self.cs.set_low();
        let _ = self.spi.write(&[0x2C]);
        let _ = self.dc.set_high();
        let staging = unsafe { core::slice::from_raw_parts_mut(self.staging, FLUSH_CHUNK) };
        for chunk in self.buffer.chunks(FLUSH_CHUNK) {
            let dst = &mut staging[..chunk.len()];
            dst.copy_from_slice(chunk);
            let _ = self.spi.write(dst);
        }
        let _ = self.cs.set_high();
    }

    pub fn clear_color(&mut self, color: Rgb565) {
        let raw = embedded_graphics_core::pixelcolor::raw::RawU16::from(color).into_inner();
        let hi = (raw >> 8) as u8;
        let lo = (raw & 0xFF) as u8;
        for i in (0..self.buffer.len()).step_by(2) {
            self.buffer[i] = hi;
            self.buffer[i + 1] = lo;
        }
        self.flush();
    }

    /// Blit a camera frame into the top WIDTH×WIDTH (240×240) region of the
    /// display buffer, then clear the rows below it (240..320) to `bg`.
    ///
    /// The camera only fills the top square; the band below it is *not* part of
    /// the live feed and nothing else writes it on the scan screens, so it must
    /// be cleared here every frame — otherwise the previous screen (e.g. the
    /// settings/About menu) bleeds through under the camera. Doing it at the
    /// blit, with direct buffer writes, also guarantees the clear regardless of
    /// which GUI draw path runs on top.
    ///
    /// Rendered as grayscale from the frame's luma plane (the camera path does
    /// no YUV→RGB conversion). The camera captures landscape; we center-crop to
    /// a square and nearest-neighbour scale into the top 240 rows.
    pub fn blit_camera_frame(&mut self, frame: &faraday_core::camera::Frame, bg: Rgb565) {
        let (fw, fh) = (frame.width, frame.height);
        let sq = fw.min(fh);
        let ox = (fw - sq) / 2;
        let oy = (fh - sq) / 2;
        for dy in 0..WIDTH {
            let sy = oy + dy * sq / WIDTH;
            for dx in 0..WIDTH {
                let sx = ox + dx * sq / WIDTH;
                let si = (sy * fw + sx) as usize;
                let y = *frame.luma.get(si).unwrap_or(&0);
                // Grayscale: replicate luma across R/G/B in RGB565.
                let r = (y >> 3) as u16;
                let g = (y >> 2) as u16;
                let b = (y >> 3) as u16;
                let rgb565 = (r << 11) | (g << 5) | b;
                let byte_idx = ((dy * WIDTH + dx) * 2) as usize;
                self.buffer[byte_idx]     = (rgb565 >> 8) as u8;
                self.buffer[byte_idx + 1] = (rgb565 & 0xFF) as u8;
            }
        }

        // Clear the band below the camera square to `bg`.
        let raw = embedded_graphics_core::pixelcolor::raw::RawU16::from(bg).into_inner();
        let hi = (raw >> 8) as u8;
        let lo = (raw & 0xFF) as u8;
        let start = (WIDTH * WIDTH * 2) as usize;
        for i in (start..self.buffer.len()).step_by(2) {
            self.buffer[i] = hi;
            self.buffer[i + 1] = lo;
        }
    }

    /// Put the panel into its lowest-power state before the device sleeps:
    /// display off (0x28) + sleep-in (0x10). The next boot re-runs `init()`.
    pub fn sleep(&mut self) {
        self.command(0x28); // display off
        self.command(0x10); // sleep in
    }

    /// Set backlight brightness. `pct` is 0–100; 0 disables the channel.
    pub fn set_backlight(&mut self, pct: u8) {
        if pct == 0 {
            let _ = self.bl.disable();
        } else {
            let duty = self.bl.get_max_duty() * pct.min(100) as u32 / 100;
            let _ = self.bl.enable();
            let _ = self.bl.set_duty(duty);
        }
    }
}

// Expose the panel to the shared event loop. The work lives in the inherent
// methods above; these forward to them through the board trait.
impl BoardDisplay for Display<'_> {
    fn flush(&mut self) {
        Display::flush(self)
    }
    fn blit_camera_frame(&mut self, frame: &faraday_core::camera::Frame, bg: Rgb565) {
        Display::blit_camera_frame(self, frame, bg)
    }
    fn set_backlight(&mut self, pct: u8) {
        Display::set_backlight(self, pct)
    }
    fn sleep(&mut self) {
        Display::sleep(self)
    }
}

impl OriginDimensions for Display<'_> {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for Display<'_> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x >= 0
                && coord.x < WIDTH as i32
                && coord.y >= 0
                && coord.y < HEIGHT as i32
            {
                let idx = ((coord.y as u32 * WIDTH + coord.x as u32) * 2) as usize;
                let raw = embedded_graphics_core::pixelcolor::raw::RawU16::from(color).into_inner();
                self.buffer[idx] = (raw >> 8) as u8;
                self.buffer[idx + 1] = (raw & 0xFF) as u8;
            }
        }
        Ok(())
    }
}
