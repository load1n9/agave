use core::cmp::{Ord, Ordering};
use core::ops::{Add, Sub};

#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct Coord {
    pub x: isize,
    pub y: isize,
}

impl Coord {
    pub fn new(x: isize, y: isize) -> Coord {
        Coord { x, y }
    }
}

impl Add<(isize, isize)> for Coord {
    type Output = Coord;

    fn add(self, rhs: (isize, isize)) -> Coord {
        Coord {
            x: self.x + rhs.0,
            y: self.y + rhs.1,
        }
    }
}

impl Sub<(isize, isize)> for Coord {
    type Output = Coord;

    fn sub(self, rhs: (isize, isize)) -> Coord {
        Coord {
            x: self.x - rhs.0,
            y: self.y - rhs.1,
        }
    }
}

impl Add<Coord> for Coord {
    type Output = Coord;

    fn add(self, rhs: Coord) -> Coord {
        Coord {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Coord> for Coord {
    type Output = Coord;

    fn sub(self, rhs: Coord) -> Coord {
        Coord {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Ord for Coord {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.y, other.y) {
            (s, o) if s > o => Ordering::Greater,
            (s, o) if s < o => Ordering::Less,
            _ => self.x.cmp(&other.x),
        }
    }
}

impl PartialOrd for Coord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Coord {}

#[derive(Clone, Copy, PartialEq, Debug, Hash)]
pub struct Rect {
    pub top_left: Coord,
    pub bottom_right: Coord,
}

impl Rect {
    pub fn width(&self) -> usize {
        (self.bottom_right.x - self.top_left.x) as usize
    }

    pub fn height(&self) -> usize {
        (self.bottom_right.y - self.top_left.y) as usize
    }
}

impl Add<Coord> for Rect {
    type Output = Rect;

    fn add(self, rhs: Coord) -> Rect {
        Rect {
            top_left: self.top_left + rhs,
            bottom_right: self.bottom_right + rhs,
        }
    }
}

impl Sub<Coord> for Rect {
    type Output = Rect;

    fn sub(self, rhs: Coord) -> Rect {
        Rect {
            top_left: self.top_left - rhs,
            bottom_right: self.bottom_right - rhs,
        }
    }
}
