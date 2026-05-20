//! QR code widget.
//!
//! Takes a data string, computes the QR matrix, and renders it as the
//! largest integer-scaled square that fits the provided rectangle, with
//! a white quiet zone so third-party scanners can lock onto it. The
//! widget is purely presentation — matrix generation lives in
//! `crate::models::encode_qr`.

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    Drawable,
};

use crate::ui::Theme;

pub struct Qr<'a> {
    pub data: &'a [u8],
    /// Error-correction level. Seed-backup QRs use `L` (smallest grid —
    /// 21×21 for 12 words) to keep hand-transcription tractable. Outbound
    /// tx / signature / address QRs use `M` (standard reliability).
    pub ec: crate::qr::encode_qr::QrEcLevel,
    /// White border in modules around the QR. Defaults to 4 (the spec
    /// recommendation for third-party scanners). Seed-backup compare
    /// screens pass a smaller value to make the matrix look larger —
    /// the user only needs to eyeball it against paper, not scan it.
    pub quiet: u32,
}

impl<'a> Qr<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        _theme: &Theme,
        rect: Rectangle,
    ) -> Result<(), D::Error> {
        let (matrix, size) = match crate::qr::encode_qr::generate_qr_matrix(self.data, self.ec) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };

        // Largest integer module size that fits in the rect.
        let max_side = rect.size.width.min(rect.size.height) as i32;
        let quiet = self.quiet as i32;
        let matrix_side = size as i32 + quiet * 2;
        let module = (max_side / matrix_side).max(1);
        let qr_side = module * matrix_side;

        let origin_x = rect.top_left.x + (rect.size.width as i32 - qr_side) / 2;
        let origin_y = rect.top_left.y + (rect.size.height as i32 - qr_side) / 2;

        // White background (including quiet zone).
        Rectangle::new(
            Point::new(origin_x, origin_y),
            Size::new(qr_side as u32, qr_side as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

        // Black modules.
        let black = PrimitiveStyle::with_fill(Rgb565::BLACK);
        let inner_x = origin_x + quiet * module;
        let inner_y = origin_y + quiet * module;
        for qr_y in 0..size {
            for qr_x in 0..size {
                if matrix[qr_y * size + qr_x] {
                    Rectangle::new(
                        Point::new(
                            inner_x + qr_x as i32 * module,
                            inner_y + qr_y as i32 * module,
                        ),
                        Size::new(module as u32, module as u32),
                    )
                    .into_styled(black)
                    .draw(display)?;
                }
            }
        }

        Ok(())
    }
}
