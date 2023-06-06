/// Represents a contiguous region in memory.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Region {
    pub start: u32,
    pub len: u32,
}

impl Region {
    pub fn new(start: u32, len: u32) -> Self {
        Self { start, len }
    }

    /// Checks if this `Region` overlaps with `rhs` `Region`.
    pub fn overlaps(&self, rhs: Region) -> bool {
        // Zero-length regions can never overlap!
        if self.len == 0 || rhs.len == 0 {
            return false;
        }

        let self_start = self.start as u64;
        let self_end = self_start + (self.len - 1) as u64;

        let rhs_start = rhs.start as u64;
        let rhs_end = rhs_start + (rhs.len - 1) as u64;

        if self_start <= rhs_start {
            self_end >= rhs_start
        } else {
            rhs_end >= self_start
        }
    }

    pub fn extend(&self, times: u32) -> Self {
        let len = self.len * times;
        Self {
            start: self.start,
            len,
        }
    }
}