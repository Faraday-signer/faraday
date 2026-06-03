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
use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::{Output, PinDriver};
use esp_idf_hal::ledc::LedcDriver;
use esp_idf_hal::spi::{SpiDeviceDriver, SpiDriver};

pub const WIDTH: u32 = 240;
pub const HEIGHT: u32 = 320;

pub struct Display<'d> {
    spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
    cs: PinDriver<'d, Output>,
    dc: PinDriver<'d, Output>,
    bl: LedcDriver<'d>,
    buffer: Vec<u8>,
}

impl<'d> Display<'d> {
    pub fn new(
        spi: SpiDeviceDriver<'d, SpiDriver<'d>>,
        cs: PinDriver<'d, Output>,
        dc: PinDriver<'d, Output>,
        bl: LedcDriver<'d>,
    ) -> Self {
        let buffer = vec![0u8; (WIDTH * HEIGHT * 2) as usize];

        let mut display = Self { spi, cs, dc, bl, buffer };
        display.init();
        display.set_backlight(30);

        display
    }

    fn command(&mut self, cmd: u8) {
        let _ = self.dc.set_low();
        let _ = self.cs.set_low();
        let _ = self.spi.write(&[cmd]);
        let _ = self.cs.set_high();
    }

    fn data_byte(&mut self, val: u8) {
        let _ = self.dc.set_high();
        let _ = self.cs.set_low();
        let _ = self.spi.write(&[val]);
        let _ = self.cs.set_high();
    }

    fn data(&mut self, vals: &[u8]) {
        let _ = self.dc.set_high();
        let _ = self.cs.set_low();
        let _ = self.spi.write(vals);
        let _ = self.cs.set_high();
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
        // must be chunked.  With DMA enabled, each write yields to FreeRTOS
        // via a semaphore — the watchdog idle task gets to run between chunks.
        // 32 764 bytes = just under the limit, 4-byte aligned, 5 chunks total.
        let _ = self.dc.set_low();
        let _ = self.cs.set_low();
        let _ = self.spi.write(&[0x2C]);
        let _ = self.dc.set_high();
        for chunk in self.buffer.chunks(32764) {
            let _ = self.spi.write(chunk);
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
    /// display buffer. The remaining rows (240..320) keep whatever the GUI
    /// drew last frame, so the scan overlay renders correctly on top.
    ///
    /// The camera captures 320×240 (landscape). We center-crop to a 240×240
    /// square (dropping 40 px from each horizontal side) and nearest-neighbour
    /// scale into the top 240 rows of the portrait display.
    pub fn blit_camera_frame(&mut self, frame: &faraday_core::camera::Frame) {
        let (fw, fh) = (frame.width, frame.height);
        let sq = fw.min(fh); // 240 for a 320×240 frame
        let ox = (fw - sq) / 2;
        let oy = (fh - sq) / 2;
        for dy in 0..WIDTH {
            let sy = oy + dy * sq / WIDTH;
            for dx in 0..WIDTH {
                let sx = ox + dx * sq / WIDTH;
                let si = ((sy * fw + sx) * 3) as usize;
                if si + 2 >= frame.rgb.len() {
                    continue;
                }
                let r = (frame.rgb[si]     >> 3) as u16;
                let g = (frame.rgb[si + 1] >> 2) as u16;
                let b = (frame.rgb[si + 2] >> 3) as u16;
                let rgb565 = (r << 11) | (g << 5) | b;
                let byte_idx = ((dy * WIDTH + dx) * 2) as usize;
                self.buffer[byte_idx]     = (rgb565 >> 8) as u8;
                self.buffer[byte_idx + 1] = (rgb565 & 0xFF) as u8;
            }
        }
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
