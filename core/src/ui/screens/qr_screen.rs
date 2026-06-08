//! QR screen template: Header + QR + single-line caption + edge-hint gutter.
//!
//! Shared shape for ShowAddress, ExportSeedQr, SignShowQr, SignMessageResult —
//! every screen that shows a QR for a counterparty to scan. The QR is the
//! hero; the caption is a truncated/formatted representation the user can
//! eyeball against the counterparty's display.

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::Rectangle,
    text::{Alignment, Text},
    Drawable,
};

use crate::ui::layout::{split_bottom, split_top};
use crate::ui::widgets::{EdgeHints, Header, HeaderKind, Qr, FOOTER_H, GUTTER_W};
use crate::ui::Theme;

pub struct QrScreen<'a> {
    pub header: HeaderKind<'a>,
    pub counter: Option<(usize, usize)>,
    pub data: &'a [u8],
    pub ec: crate::qr::encode_qr::QrEcLevel,
    pub caption: Option<&'a str>,
    pub edge_hints: EdgeHints,
}

impl<'a> QrScreen<'a> {
    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
    ) -> Result<(), D::Error> {
        let screen = Rectangle::new(Point::zero(), Size::new(theme.width, theme.height));
        display.fill_solid(&screen, theme.bg)?;

        let (header_rect, rest) = split_top(screen, theme.header_h as i32);

        let body_area = if self.edge_hints.is_empty() {
            rest
        } else {
            Rectangle::new(
                rest.top_left,
                Size::new(rest.size.width - GUTTER_W, rest.size.height - FOOTER_H),
            )
        };
        let (body_rect, _footer_rect) = split_bottom(body_area, 0);

        let kind = match self.header {
            HeaderKind::Title(t) => HeaderKind::Title(t),
            HeaderKind::Brand => HeaderKind::Brand,
        };
        Header {
            kind,
            counter: self.counter,
            right_label: None,
        }
        .draw(display, theme, header_rect)?;

        // Carve off a caption band at the bottom of the body. If no caption
        // is provided, the QR uses the full body height.
        let caption_h = if self.caption.is_some() { 24 } else { 0 };
        let (qr_rect, caption_rect) = split_bottom(body_rect, caption_h);

        Qr {
            data: self.data,
            ec: self.ec,
            quiet: 4,
        }
        .draw(display, theme, qr_rect)?;

        if let Some(caption) = self.caption {
            let baseline = caption_rect.top_left.y + caption_rect.size.height as i32 - 8;
            let x = caption_rect.top_left.x + caption_rect.size.width as i32 / 2;
            Text::with_alignment(
                caption,
                Point::new(x, baseline),
                theme.style_sm(theme.muted),
                Alignment::Center,
            )
            .draw(display)?;
        }

        if !self.edge_hints.is_empty() {
            let gutter = Rectangle::new(
                Point::new(
                    rest.top_left.x + rest.size.width as i32 - GUTTER_W as i32,
                    rest.top_left.y,
                ),
                Size::new(GUTTER_W, rest.size.height),
            );
            self.edge_hints.draw(display, theme, gutter)?;
        }

        Ok(())
    }
}
