//! Faraday UI library.
//!
//! A thin, opinionated widget layer on top of `embedded-graphics`.
//! Built around three registers:
//!   - **List** for navigation (several choices).
//!   - **Card** for commitment (one fact per screen).
//!   - **Input** for live data entry (grid, scanner, wheel).
//!
//! Designed to be extracted into its own crate once the API stabilises.
//! All widgets are data + `draw(display, theme, rect)` — state lives in the
//! caller. Themes encode every parameter that varies across screen sizes,
//! so the same widgets render on any `DrawTarget`.

pub mod layout;
pub mod screens;
pub mod tokens;
pub mod widgets;

pub use layout::Insets;
pub use tokens::Theme;
