//! UI components: status bar.

use embedded_graphics::{
    mono_font::{ascii::FONT_9X15_BOLD, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
};

use crate::gui::colors;
use crate::ui::Theme;

/// Status bar at top of screen.
pub fn draw_status_bar<D: DrawTarget<Color = Rgb565>>(
    display: &mut D,
    theme: &Theme,
    title: &str,
    seed_loaded: bool,
) -> Result<(), D::Error> {
    Rectangle::new(Point::zero(), Size::new(theme.width, 20))
        .into_styled(PrimitiveStyle::with_fill(colors::BG_DARK))
        .draw(display)?;

    Rectangle::new(Point::new(0, 19), Size::new(theme.width, 1))
        .into_styled(PrimitiveStyle::with_fill(colors::BORDER_DEFAULT))
        .draw(display)?;

    let center_x = (theme.width / 2) as i32;
    let style = MonoTextStyle::new(&FONT_9X15_BOLD, colors::TEXT_SECONDARY);
    Text::with_alignment(title, Point::new(center_x, 15), style, Alignment::Center).draw(display)?;

    if seed_loaded {
        let indicator_x = (theme.width - 16) as i32;
        Rectangle::new(Point::new(indicator_x, 6), Size::new(8, 8))
            .into_styled(PrimitiveStyle::with_fill(colors::SUCCESS))
            .draw(display)?;
    }

    Ok(())
}
