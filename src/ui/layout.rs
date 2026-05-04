//! Layout primitives.
//!
//! Every widget draws into a `Rectangle`. These helpers carve rectangles
//! from a parent box — insets, vertical splits, bottom reservations —
//! so widgets never hardcode absolute coordinates.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

/// Carve a fixed-height band off the top. Returns (top, rest).
pub fn split_top(rect: Rectangle, top_h: i32) -> (Rectangle, Rectangle) {
    let top_h = top_h.max(0);
    let top = Rectangle::new(rect.top_left, Size::new(rect.size.width, top_h as u32));
    let rest = Rectangle::new(
        Point::new(rect.top_left.x, rect.top_left.y + top_h),
        Size::new(
            rect.size.width,
            (rect.size.height as i32 - top_h).max(0) as u32,
        ),
    );
    (top, rest)
}

/// Carve a fixed-height band off the bottom. Returns (rest, bottom).
pub fn split_bottom(rect: Rectangle, bottom_h: i32) -> (Rectangle, Rectangle) {
    let bottom_h = bottom_h.max(0);
    let body_h = (rect.size.height as i32 - bottom_h).max(0) as u32;
    let rest = Rectangle::new(rect.top_left, Size::new(rect.size.width, body_h));
    let bottom = Rectangle::new(
        Point::new(rect.top_left.x, rect.top_left.y + body_h as i32),
        Size::new(rect.size.width, bottom_h as u32),
    );
    (rest, bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: u32, h: u32) -> Rectangle {
        Rectangle::new(Point::new(x, y), Size::new(w, h))
    }

    #[test]
    fn split_top_partitions_the_rect() {
        let r = rect(0, 0, 240, 240);
        let (top, rest) = split_top(r, 30);
        assert_eq!(top.size, Size::new(240, 30));
        assert_eq!(rest.top_left, Point::new(0, 30));
        assert_eq!(rest.size, Size::new(240, 210));
    }

    #[test]
    fn split_bottom_partitions_the_rect() {
        let r = rect(0, 0, 240, 240);
        let (body, bottom) = split_bottom(r, 26);
        assert_eq!(body.size, Size::new(240, 214));
        assert_eq!(bottom.top_left, Point::new(0, 214));
        assert_eq!(bottom.size, Size::new(240, 26));
    }

    #[test]
    fn split_top_clamps_when_band_exceeds_rect() {
        let r = rect(0, 0, 100, 20);
        let (top, rest) = split_top(r, 50);
        assert_eq!(top.size.height, 50);
        assert_eq!(rest.size.height, 0);
    }
}
