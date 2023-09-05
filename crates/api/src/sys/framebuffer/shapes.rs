use core::ops::{Add, Sub};
use core::cmp::{Ord, Ordering};

#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct Coordinate {
    pub x: isize,
    pub y: isize,
}

impl Coordinate {
    pub fn new(x: isize, y: isize) -> Coordinate {
        Coordinate { x, y }
    }
}

impl Add<(isize, isize)> for Coordinate {
    type Output = Coordinate;

    fn add(self, rhs: (isize, isize)) -> Coordinate {
        Coordinate { x: self.x + rhs.0, y: self.y + rhs.1 }
    }
}

impl Sub<(isize, isize)> for Coordinate {
    type Output = Coordinate;

    fn sub(self, rhs: (isize, isize)) -> Coordinate {
        Coordinate { x: self.x - rhs.0, y: self.y - rhs.1 }
    }
}

impl Add<Coordinate> for Coordinate {
    type Output = Coordinate;

    fn add(self, rhs: Coordinate) -> Coordinate {
        Coordinate {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Coordinate> for Coordinate {
    type Output = Coordinate;

    fn sub(self, rhs: Coordinate) -> Coordinate {
        Coordinate {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Ord for Coordinate {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.y, other.y) {
            (s, o) if s > o => Ordering::Greater,
            (s, o) if s < o => Ordering::Less,
            _ => self.x.cmp(&other.x),
        }
    }
}

impl PartialOrd for Coordinate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Coordinate { }


#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct Rectangle {
    /// The top-left point
    pub top_left: Coordinate,
    /// The bottom-right point
    pub bottom_right: Coordinate,
}

impl Rectangle {
    /// Returns the width of this Rectangle.
    pub fn width(&self) -> usize {
        (self.bottom_right.x - self.top_left.x) as usize
    }

    /// Returns the height of this Rectangle.
    pub fn height(&self) -> usize {
        (self.bottom_right.y - self.top_left.y) as usize
    }
}

impl Add<Coordinate> for Rectangle {
    type Output = Rectangle;

    fn add(self, rhs: Coordinate) -> Rectangle {
        Rectangle {
            top_left: self.top_left + rhs,
            bottom_right: self.bottom_right + rhs,
        }
    }
}

impl Sub<Coordinate> for Rectangle {
    type Output = Rectangle;

    fn sub(self, rhs: Coordinate) -> Rectangle {
        Rectangle {
            top_left: self.top_left - rhs,
            bottom_right: self.bottom_right - rhs,
        }
    }
}