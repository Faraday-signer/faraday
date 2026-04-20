//! Button bar: 3-slot legend of what each hardware button does on the
//! current screen. Keeps input grammar discoverable without on-device
//! documentation.
//!
//! Slots map to the Waveshare HAT buttons:
//!   - left:      Key3 (back)
//!   - middle:    Key2 (secondary / contextual)
//!   - right:     Key1 (confirm — the commit action)
//!
//! Only the `confirm` label renders in `theme.accent`. Everything else is
//! muted — reserves cyan for "you are about to commit."

use embedded_graphics::{
    geometry::{Point, Size},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
    Drawable,
};

use crate::ui::Theme;

pub struct ButtonBar<'a> {
    pub back: Option<&'a str>,
    pub secondary: Option<&'a str>,
    pub confirm: Option<&'a str>,
}

impl<'a> ButtonBar<'a> {
    pub const fn new() -> Self {
        Self { back: None, secondary: None, confirm: None }
    }

    pub const fn back(mut self, label: &'a str) -> Self {
        self.back = Some(label);
        self
    }

    pub const fn secondary(mut self, label: &'a str) -> Self {
        self.secondary = Some(label);
        self
    }

    pub const fn confirm(mut self, label: &'a str) -> Self {
        self.confirm = Some(label);
        self
    }

    pub fn draw<D: DrawTarget<Color = Rgb565>>(
        &self,
        display: &mut D,
        theme: &Theme,
        rect: Rectangle,
    ) -> Result<(), D::Error> {
        // Top hairline.
        Rectangle::new(rect.top_left, Size::new(rect.size.width, 1))
            .into_styled(PrimitiveStyle::with_fill(theme.border))
            .draw(display)?;

        let baseline = rect.top_left.y + rect.size.height as i32 - 6;
        let left_x = rect.top_left.x + theme.space_md;
        let right_x = rect.top_left.x + rect.size.width as i32 - theme.space_md;
        let mid_x = rect.top_left.x + rect.size.width as i32 / 2;

        if let Some(label) = self.back {
            Text::with_alignment(
                label,
                Point::new(left_x, baseline),
                theme.style_sm(theme.muted),
                Alignment::Left,
            )
            .draw(display)?;
        }

        if let Some(label) = self.secondary {
            Text::with_alignment(
                label,
                Point::new(mid_x, baseline),
                theme.style_sm(theme.muted),
                Alignment::Center,
            )
            .draw(display)?;
        }

        if let Some(label) = self.confirm {
            Text::with_alignment(
                label,
                Point::new(right_x, baseline),
                theme.style_sm(theme.accent),
                Alignment::Right,
            )
            .draw(display)?;
        }

        Ok(())
    }
}
