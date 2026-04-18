//! ST7789 SPI display driver for Waveshare 1.3" LCD HAT (240x240).
//!
//! Pin mapping (BCM numbering):
//!   DC  = GPIO 25
//!   RST = GPIO 27
//!   BL  = GPIO 24
//!   SPI = CE0 (bus 0, chip select 0)

use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{raw::RawData, Rgb565},
    Pixel,
};
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::thread;
use std::time::Duration;

pub const WIDTH: u32 = 240;
pub const HEIGHT: u32 = 240;

const DC_PIN: u8 = 25;
const RST_PIN: u8 = 27;
const BL_PIN: u8 = 24;
const SPI_SPEED_HZ: u32 = 40_000_000;

pub struct ST7789 {
    spi: Spi,
    dc: OutputPin,
    rst: OutputPin,
    bl: OutputPin,
    /// Framebuffer: 240*240 pixels * 2 bytes each (RGB565 big-endian)
    buffer: Vec<u8>,
}

impl ST7789 {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let gpio = Gpio::new()?;
        let dc = gpio.get(DC_PIN)?.into_output();
        let rst = gpio.get(RST_PIN)?.into_output();
        let bl = gpio.get(BL_PIN)?.into_output();
        let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, SPI_SPEED_HZ, Mode::Mode0)?;

        let buffer = vec![0u8; (WIDTH * HEIGHT * 2) as usize];

        let mut display = ST7789 { spi, dc, rst, bl, buffer };
        display.reset();
        display.init();
        display.set_backlight(true);

        Ok(display)
    }

    fn command(&mut self, cmd: u8) {
        self.dc.set_low();
        let _ = self.spi.write(&[cmd]);
    }

    fn data_byte(&mut self, val: u8) {
        self.dc.set_high();
        let _ = self.spi.write(&[val]);
    }

    fn data(&mut self, vals: &[u8]) {
        self.dc.set_high();
        let _ = self.spi.write(vals);
    }

    fn reset(&mut self) {
        self.rst.set_high();
        thread::sleep(Duration::from_millis(10));
        self.rst.set_low();
        thread::sleep(Duration::from_millis(10));
        self.rst.set_high();
        thread::sleep(Duration::from_millis(10));
    }

    fn init(&mut self) {
        // MADCTL: 0x70 = MX + MV + ML
        self.command(0x36);
        self.data_byte(0x70);

        // COLMOD: 16-bit color (RGB565)
        self.command(0x3A);
        self.data_byte(0x05);

        // Porch setting
        self.command(0xB2);
        self.data(&[0x0C, 0x0C, 0x00, 0x33, 0x33]);

        // Gate control
        self.command(0xB7);
        self.data_byte(0x35);

        // VCOM
        self.command(0xBB);
        self.data_byte(0x19);

        // LCM control
        self.command(0xC0);
        self.data_byte(0x2C);

        // VDV and VRH enable
        self.command(0xC2);
        self.data_byte(0x01);

        // VRH
        self.command(0xC3);
        self.data_byte(0x12);

        // VDV
        self.command(0xC4);
        self.data_byte(0x20);

        // Frame rate: 60Hz
        self.command(0xC6);
        self.data_byte(0x0F);

        // Power control
        self.command(0xD0);
        self.data(&[0xA4, 0xA1]);

        // Positive gamma
        self.command(0xE0);
        self.data(&[
            0xD0, 0x04, 0x0D, 0x11, 0x13, 0x2B, 0x3F,
            0x54, 0x4C, 0x18, 0x0D, 0x0B, 0x1F, 0x23,
        ]);

        // Negative gamma
        self.command(0xE1);
        self.data(&[
            0xD0, 0x04, 0x0C, 0x11, 0x13, 0x2C, 0x3F,
            0x44, 0x51, 0x2F, 0x1F, 0x1F, 0x20, 0x23,
        ]);

        // Color inversion ON
        self.command(0x21);

        // Sleep out
        self.command(0x11);
        thread::sleep(Duration::from_millis(50));

        // Display on
        self.command(0x29);
        thread::sleep(Duration::from_millis(50));
    }

    fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) {
        // Column address
        self.command(0x2A);
        self.data(&[
            (x0 >> 8) as u8, (x0 & 0xFF) as u8,
            ((x1 - 1) >> 8) as u8, ((x1 - 1) & 0xFF) as u8,
        ]);

        // Row address
        self.command(0x2B);
        self.data(&[
            (y0 >> 8) as u8, (y0 & 0xFF) as u8,
            ((y1 - 1) >> 8) as u8, ((y1 - 1) & 0xFF) as u8,
        ]);

        // Memory write
        self.command(0x2C);
    }

    /// Flush the internal framebuffer to the display.
    pub fn flush(&mut self) {
        self.set_window(0, 0, WIDTH as u16, HEIGHT as u16);
        self.dc.set_high();
        let _ = self.spi.write(&self.buffer);
    }

    /// Blit a webcam RGB frame into the display buffer using a center-crop +
    /// nearest-neighbor scale, rotated 90° clockwise. The OV5647 is mounted
    /// with its native frame landscape while the display is held portrait,
    /// so the captured image arrives on its side; rotating during blit keeps
    /// the QR decoder seeing the original (rotation-invariant) frame while
    /// giving the user a correctly-oriented preview.
    ///
    /// Movement sanity check: with this rotation, moving your hand RIGHT in
    /// front of the camera should appear to move RIGHT on screen, and DOWN
    /// should move DOWN. If either axis looks mirrored, we need to add an
    /// additional flip — see the note on `FLIP_HORIZONTAL` below.
    pub fn blit_camera_frame(&mut self, frame: &crate::camera::Frame) {
        // Center-crop to a square so rotation + scale don't stretch.
        let sq = frame.width.min(frame.height);
        let ox = (frame.width - sq) / 2;
        let oy = (frame.height - sq) / 2;
        // 90° CW reverse-mapping within the square: rotated(x, y) reads from
        // original(y, sq-1-x). See the unit-test-style derivation in the
        // commit message.
        for dy in 0..HEIGHT {
            // `sq_y` is the row we'd sample from at this display y BEFORE
            // rotation — with rotation it becomes the x-axis into the frame.
            let sq_y = dy * sq / HEIGHT;
            let sx = ox + sq_y;
            for dx in 0..WIDTH {
                let sq_x = dx * sq / WIDTH;
                let sy = oy + sq - 1 - sq_x;
                let si = ((sy * frame.width + sx) * 3) as usize;
                if si + 2 >= frame.rgb.len() {
                    continue;
                }
                let r = (frame.rgb[si] >> 3) as u16;
                let g = (frame.rgb[si + 1] >> 2) as u16;
                let b = (frame.rgb[si + 2] >> 3) as u16;
                let rgb565 = (r << 11) | (g << 5) | b;
                let idx = ((dy * WIDTH + dx) * 2) as usize;
                self.buffer[idx] = (rgb565 >> 8) as u8;
                self.buffer[idx + 1] = (rgb565 & 0xFF) as u8;
            }
        }
    }

    /// Fill the entire display with a single RGB565 color.
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

    pub fn set_backlight(&mut self, on: bool) {
        if on { self.bl.set_high(); } else { self.bl.set_low(); }
    }

    pub fn cleanup(&mut self) {
        self.set_backlight(false);
    }
}

impl OriginDimensions for ST7789 {
    fn size(&self) -> Size {
        Size::new(WIDTH, HEIGHT)
    }
}

impl DrawTarget for ST7789 {
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
