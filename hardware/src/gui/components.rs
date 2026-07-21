//! UI components: status bar.

use embedded_graphics::{
    mono_font::{ascii::FONT_9X15_BOLD, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

use crate::gui::colors;

/// Status bar at top of screen.
///
/// Only reachable from the `not(simulator)`-and-`not(linux)` fallback
/// `draw_message` in `screens.rs`. Marked `dead_code` so the simulator/
/// Linux feature matrix CI uses doesn't complain.
#[allow(dead_code)]
pub fn draw_status_bar<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    title: &str,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    Rectangle::new(Point::zero(), Size::new(240, 20))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_DARK))
        .draw(display)?;

    Rectangle::new(Point::new(0, 19), Size::new(240, 1))
        .into_styled(PrimitiveStyle::with_fill(colors::BORDER_DEFAULT))
        .draw(display)?;

    let style = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_SECONDARY);
    Text::with_alignment(title, Point::new(120, 15), style, Alignment::Center).draw(display)?;

    if seed_loaded {
        Rectangle::new(Point::new(224, 6), Size::new(8, 8))
            .into_styled(PrimitiveStyle::with_fill(colors::SUCCESS))
            .draw(display)?;
    }

    Ok(())
}
