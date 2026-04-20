//! Layout primitives.
//!
//! Every widget draws into a `Rectangle`. These helpers carve rectangles
//! from a parent box — insets, vertical splits, bottom reservations —
//! so widgets never hardcode absolute coordinates.

use embedded_graphics::{
    geometry::{Point, Size},
    primitives::Rectangle,
};

#[derive(Debug, Clone, Copy)]
pub struct Insets {
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub left: i32,
}

impl Insets {
    pub const fn all(v: i32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }

    pub const fn symmetric(v: i32, h: i32) -> Self {
        Self { top: v, right: h, bottom: v, left: h }
    }
}

/// Shrink a rectangle by insets.
pub fn inset(rect: Rectangle, insets: Insets) -> Rectangle {
    let x = rect.top_left.x + insets.left;
    let y = rect.top_left.y + insets.top;
    let w = (rect.size.width as i32 - insets.left - insets.right).max(0) as u32;
    let h = (rect.size.height as i32 - insets.top - insets.bottom).max(0) as u32;
    Rectangle::new(Point::new(x, y), Size::new(w, h))
}

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
        Point::new(
            rect.top_left.x,
            rect.top_left.y + body_h as i32,
        ),
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
    fn inset_shrinks_from_all_sides() {
        let r = rect(10, 10, 100, 100);
        let out = inset(r, Insets::all(5));
        assert_eq!(out.top_left, Point::new(15, 15));
        assert_eq!(out.size, Size::new(90, 90));
    }

    #[test]
    fn inset_asymmetric() {
        let r = rect(0, 0, 100, 100);
        let out = inset(
            r,
            Insets { top: 2, right: 4, bottom: 6, left: 8 },
        );
        assert_eq!(out.top_left, Point::new(8, 2));
        assert_eq!(out.size, Size::new(100 - 8 - 4, 100 - 2 - 6));
    }

    #[test]
    fn inset_clamps_to_zero() {
        let r = rect(0, 0, 10, 10);
        let out = inset(r, Insets::all(20));
        assert_eq!(out.size, Size::new(0, 0));
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
