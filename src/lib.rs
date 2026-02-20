use std::{
    cmp::{max, min},
    str::FromStr,
};

use bits::BitArray;

#[derive(Clone)]
pub struct BitGrid {
    bits: BitArray,
    min_x: i64,
    min_y: i64,
    max_x: i64,
    max_y: i64,
}

impl Default for BitGrid {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl FromIterator<(i64, i64, bool)> for BitGrid {
    fn from_iter<T: IntoIterator<Item = (i64, i64, bool)>>(iter: T) -> Self {
        let mut result = BitGrid::default();
        for (x, y, value) in iter {
            result.set(x, y, value);
        }
        result
    }
}

impl FromStr for BitGrid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self::default();
        for (y, row) in s.split("\n").enumerate() {
            for (x, cell) in row.char_indices() {
                let value = match cell {
                    '1' | 'X' | '*' => true,
                    '0' | 'O' | '.' => false,
                    _ => return Err(anyhow::anyhow!("Illegal char: {cell}")),
                };
                result.set(x as i64, y as i64, value);
            }
        }
        Ok(result)
    }
}

impl BitGrid {
    pub fn new(min_x: i64, max_x: i64, min_y: i64, max_y: i64) -> Self {
        let num_zeros = span(min_x, max_x) * span(min_y, max_y);
        Self {
            min_x,
            max_x,
            min_y,
            max_y,
            bits: BitArray::zeros(num_zeros as u64),
        }
    }

    pub fn is_set(&self, x: i64, y: i64) -> Option<bool> {
        self.index_1d(x, y).map(|i| self.bits.is_set(i))
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, i64, bool)> {
        let xy: CoordIter = self.into();
        xy.map(|(x, y)| (x, y, self.is_set(x, y).unwrap()))
    }

    pub fn set(&mut self, x: i64, y: i64, value: bool) {
        match self.index_1d(x, y) {
            Some(i) => {
                self.bits.set(i, value);
            }
            None => {
                let min_x = min(x, self.min_x);
                let max_x = max(x, self.max_x);
                let min_y = min(y, self.min_y);
                let max_y = max(y, self.max_y);
                let mut new_self = Self::new(min_x, max_x, min_y, max_y);
                for (x, y, value) in self.iter() {
                    new_self.bits.set(self.unchecked_index_1d(x, y), value);
                }
                new_self.bits.set(self.unchecked_index_1d(x, y), value);
                std::mem::swap(&mut new_self, self);
            }
        }
    }

    pub fn width(&self) -> i64 {
        span(self.min_x, self.max_x)
    }

    pub fn height(&self) -> i64 {
        span(self.min_y, self.max_y)
    }

    fn index_1d(&self, x: i64, y: i64) -> Option<u64> {
        if self.min_x <= x && x <= self.max_x && self.min_y <= y && y <= self.max_y {
            Some(self.unchecked_index_1d(x, y))
        } else {
            None
        }
    }

    fn unchecked_index_1d(&self, x: i64, y: i64) -> u64 {
        let grid_x = x - self.min_x;
        let grid_y = y - self.min_y;
        (grid_y * self.width() + grid_x) as u64
    }
}

struct CoordIter {
    max_y: i64,
    min_x: i64,
    max_x: i64,
    x: i64,
    y: i64,
}

impl From<&BitGrid> for CoordIter {
    fn from(value: &BitGrid) -> Self {
        Self {
            max_y: value.max_y,
            min_x: value.min_x,
            max_x: value.max_x,
            x: value.min_x,
            y: value.min_y,
        }
    }
}

impl Iterator for CoordIter {
    type Item = (i64, i64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y > self.max_y {
            None
        } else {
            let result = Some((self.x, self.y));
            self.x += 1;
            if self.x > self.max_x {
                self.x = self.min_x;
                self.y += 1;
            }
            result
        }
    }
}

fn span(min: i64, max: i64) -> i64 {
    max - min + 1
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let basic = "1101\n1011\n0010\n";
        let expected = basic.parse::<BitGrid>().unwrap();
        assert_eq!(expected.height(), 3);
        assert_eq!(expected.width(), 4);
        for (x, y, value) in [
            (0, 0, true),
            (1, 0, true),
            (2, 0, false),
            (3, 0, true),
            (0, 1, true),
            (1, 1, false),
            (2, 1, true),
            (3, 1, true),
            (0, 2, false),
            (1, 2, false),
            (2, 2, true),
            (3, 2, false),
        ] {
            assert_eq!(expected.is_set(x, y).unwrap(), value);
        }

        for (x, y) in [(-1, 0), (3, 3), (1, 3), (4, 1), (1, -3)] {
            assert_eq!(expected.is_set(x, y), None);
        }
    }
}
