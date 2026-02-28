use std::{
    cmp::{max, min},
    fmt::Display,
    str::FromStr,
};

use bits::BitArray;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GrowingBitGrid {
    bits: BitArray,
    min_x: i64,
    min_y: i64,
    max_x: i64,
    max_y: i64,
}

impl Default for GrowingBitGrid {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl FromIterator<(i64, i64, bool)> for GrowingBitGrid {
    fn from_iter<T: IntoIterator<Item = (i64, i64, bool)>>(iter: T) -> Self {
        let mut result = GrowingBitGrid::default();
        for (x, y, value) in iter {
            result.set(x, y, value);
        }
        result
    }
}

impl FromStr for GrowingBitGrid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Self::default();
        for (y, row) in s.trim().split("\n").enumerate() {
            for (x, cell) in row.trim().char_indices() {
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

impl Display for GrowingBitGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (x, y, value) in self.iter() {
            if y > self.min_y && x == self.min_x {
                write!(f, "\n")?;
            }
            let c = if value { '1' } else { '0' };
            write!(f, "{c}")?;
        }
        Ok(())
    }
}

impl GrowingBitGrid {
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

    fn with_bits(&self, alt_bits: BitArray) -> Self {
        Self {
            min_x: self.min_x,
            max_x: self.max_x,
            min_y: self.min_y,
            max_y: self.max_y,
            bits: alt_bits,
        }
    }

    pub fn zero_clone(&self) -> Self {
        Self::new(self.min_x, self.max_x, self.min_y, self.max_y)
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, i64, bool)> {
        let xy: CoordIter = CoordIter::from(self);
        xy.map(|(x, y)| (x, y, self.is_set(x, y)))
    }

    pub fn ones(&self) -> impl Iterator<Item = (i64, i64)> {
        let xy: CoordIter = CoordIter::from(self);
        xy.filter(|(x, y)| self.is_set(*x, *y))
    }

    pub fn ones_touching_zeros(&self) -> impl Iterator<Item = (i64, i64)> {
        self.ones().filter(|(x, y)| {
            self.manhattan_neighbors(*x, *y)
                .filter(|(_, _, value)| *value)
                .count()
                < 4
        })
    }

    pub fn manhattan_neighbors(&self, x: i64, y: i64) -> impl Iterator<Item = (i64, i64, bool)> {
        ManhattanIter::new(x, y, self)
    }

    pub fn count_bits_on(&self) -> u64 {
        self.bits.count_bits_on()
    }

    pub fn in_bounds(&self, x: i64, y: i64) -> bool {
        self.index_1d(x, y).is_some()
    }

    pub fn is_set(&self, x: i64, y: i64) -> bool {
        self.index_1d(x, y).map_or(false, |i| self.bits.is_set(i))
    }

    pub fn set(&mut self, x: i64, y: i64, value: bool) {
        match self.index_1d(x, y) {
            Some(i) => {
                self.bits.set(i, value);
            }
            None => {
                let min_x = if x < self.min_x {
                    x - self.width()
                } else {
                    self.min_x
                };
                let max_x = if x > self.max_x {
                    x + self.width()
                } else {
                    self.max_x
                };
                let min_y = if y < self.min_y {
                    y - self.height()
                } else {
                    self.min_y
                };
                let max_y = if y > self.max_y {
                    y + self.height()
                } else {
                    self.max_y
                };
                self.resize(min_x, max_x, min_y, max_y);
                self.bits.set(self.unchecked_index_1d(x, y), value);
            }
        }
    }

    pub fn match_sizes(&mut self, other: &mut Self) {
        let min_x = min(self.min_x, other.min_x);
        let max_x = max(self.max_x, other.max_x);
        let min_y = min(self.min_y, other.min_y);
        let max_y = max(self.max_y, other.max_y);
        self.resize(min_x, max_x, min_y, max_y);
        other.resize(min_x, max_x, min_y, max_y);
    }

    fn resize(&mut self, min_x: i64, max_x: i64, min_y: i64, max_y: i64) {
        if min_x != self.min_x || max_x != self.max_x || min_y != self.min_y || max_y != self.max_y
        {
            let mut new_self = Self::new(min_x, max_x, min_y, max_y);
            for (x, y, value) in self.iter() {
                new_self.bits.set(new_self.unchecked_index_1d(x, y), value);
            }
            std::mem::swap(&mut new_self, self);
        }
    }

    pub fn width(&self) -> i64 {
        span(self.min_x, self.max_x)
    }

    pub fn height(&self) -> i64 {
        span(self.min_y, self.max_y)
    }

    pub fn matching_dimensions(&self, other: &Self) -> bool {
        self.min_x == other.min_x
            && self.min_y == other.min_y
            && self.max_x == other.max_x
            && self.max_y == other.max_y
    }

    pub fn intersection(&self, other: &Self) -> Option<GrowingBitGrid> {
        if self.matching_dimensions(other) {
            Some(self.with_bits(&self.bits & &other.bits))
        } else {
            None
        }
    }

    pub fn union(&self, other: &Self) -> Option<GrowingBitGrid> {
        if self.matching_dimensions(other) {
            Some(self.with_bits(&self.bits | &other.bits))
        } else {
            None
        }
    }

    pub fn overlapping_counts(&self, other: &Self) -> Option<u64> {
        self.intersection(other)
            .map(|overlaps| overlaps.count_bits_on())
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

    pub fn x_min_x_max_y_min_y_max(&self) -> (i64, i64, i64, i64) {
        (self.min_x, self.max_x, self.min_y, self.max_y)
    }
}

struct CoordIter {
    max_y: i64,
    min_x: i64,
    max_x: i64,
    x: i64,
    y: i64,
}

impl CoordIter {
    fn from(value: &GrowingBitGrid) -> Self {
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

const MANHATTAN_OFFSETS: [(i64, i64); 4] = [(-1, 0), (0, -1), (1, 0), (0, 1)];

struct ManhattanIter<'a> {
    base_x: i64,
    base_y: i64,
    offset: usize,
    grid: &'a GrowingBitGrid,
}

impl<'a> ManhattanIter<'a> {
    fn new(x: i64, y: i64, grid: &'a GrowingBitGrid) -> Self {
        Self {
            base_x: x,
            base_y: y,
            offset: 0,
            grid,
        }
    }
}

impl<'a> Iterator for ManhattanIter<'a> {
    type Item = (i64, i64, bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == MANHATTAN_OFFSETS.len() {
            None
        } else {
            let (offset_x, offset_y) = MANHATTAN_OFFSETS[self.offset];
            let x = self.base_x + offset_x;
            let y = self.base_y + offset_y;
            self.offset += 1;
            Some((x, y, self.grid.is_set(x, y)))
        }
    }
}

fn span(min: i64, max: i64) -> i64 {
    max - min + 1
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::*;

    #[test]
    fn test_from_str() {
        let grid_str = "1101000\n1011000\n0010000\n";
        let grid = grid_str.parse::<GrowingBitGrid>().unwrap();
        assert_eq!(format!("{grid}\n"), grid_str);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.width(), 7);
        assert_eq!(grid.count_bits_on(), 7);
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
            (4, 1, false),
        ] {
            assert_eq!(grid.is_set(x, y), value);
            assert!(grid.in_bounds(x, y));
        }

        // out of bounds - still false
        for (x, y) in [(-1, 0), (3, 3), (1, 3), (1, -3)] {
            assert_eq!(grid.is_set(x, y), false);
            assert!(!grid.in_bounds(x, y));
        }

        let zero_grid = grid.zero_clone();
        assert!(grid.matching_dimensions(&zero_grid));
        assert_eq!(zero_grid.width(), grid.width());
        assert_eq!(zero_grid.height(), grid.height());
        assert!(zero_grid.iter().all(|(_, _, value)| !value));
    }

    #[test]
    fn test_from_iter() {
        let pvs = [(2, 2, true), (-1, 3, false), (3, -2, true)];
        let test_points = pvs
            .iter()
            .map(|(x, y, value)| ((*x, *y), *value))
            .collect::<HashMap<_, _>>();
        let grid = pvs.iter().copied().collect::<GrowingBitGrid>();
        assert_eq!(grid.count_bits_on(), 2);
        assert_eq!(grid.width(), 9);
        assert_eq!(grid.height(), 10);
        for (x, y, value) in grid.iter() {
            match test_points.get(&(x, y)) {
                Some(expected) => {
                    assert_eq!(value, *expected);
                }
                None => {
                    assert_eq!(value, false);
                }
            }
        }

        let zeros = grid.zero_clone();
        assert!(zeros.matching_dimensions(&grid));
        assert_eq!(
            zeros.x_min_x_max_y_min_y_max(),
            grid.x_min_x_max_y_min_y_max()
        );
    }

    #[test]
    fn test_overlapping_counts() {
        for (one, two, count) in [
            ("101\n010", "110\n110", 2),
            ("101\n010", "010\n101", 0),
            ("101\n010", "101\n010", 3),
            ("111\n111", "111\n111", 6),
            ("000\n000", "000\n000", 0),
        ] {
            let one = one.parse::<GrowingBitGrid>().unwrap();
            let two = two.parse::<GrowingBitGrid>().unwrap();
            assert_eq!(count, one.overlapping_counts(&two).unwrap());
        }
    }

    #[test]
    fn test_match_sizes() {
        let mut one = "01\n10\n11".parse::<GrowingBitGrid>().unwrap();
        let mut two = "101\n110".parse::<GrowingBitGrid>().unwrap();
        one.match_sizes(&mut two);
        let one_big = "010\n100\n110".parse::<GrowingBitGrid>().unwrap();
        let two_big = "101\n110\n000".parse::<GrowingBitGrid>().unwrap();
        assert_eq!(one, one_big);
        assert_eq!(two, two_big);
    }

    #[test]
    fn test_manhattan_iter() {
        let test_grid = "010\n100\n101".parse::<GrowingBitGrid>().unwrap();
        for (x, y, neighbors) in [
            (
                0,
                0,
                vec![(-1, 0, false), (0, -1, false), (1, 0, true), (0, 1, true)],
            ),
            (
                1,
                1,
                vec![(0, 1, true), (1, 0, true), (2, 1, false), (1, 2, false)],
            ),
            (
                1,
                2,
                vec![(0, 2, true), (1, 1, false), (2, 2, true), (1, 3, false)],
            ),
        ] {
            let actual = test_grid.manhattan_neighbors(x, y).collect::<Vec<_>>();
            assert_eq!(actual, neighbors);
        }
    }

    #[test]
    fn test_ones_touching_zeros() {
        let test_grid = "
        10100000100000011
        11111010101011011
        10110000001000011
        "
        .parse::<GrowingBitGrid>()
        .unwrap();
        let found = test_grid.ones_touching_zeros().collect::<BTreeSet<_>>();
        let expected = [
            (0, 0),
            (2, 0),
            (8, 0),
            (15, 0),
            (16, 0),
            (0, 1),
            (1, 1),
            (3, 1),
            (4, 1),
            (6, 1),
            (8, 1),
            (10, 1),
            (12, 1),
            (13, 1),
            (15, 1),
            (16, 1),
            (0, 2),
            (2, 2),
            (3, 2),
            (10, 2),
            (15, 2),
            (16, 2),
        ]
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
        assert_eq!(expected, found);
        let one_count = test_grid.count_bits_on();
        assert_eq!(found.len() as u64 + 1, one_count);
    }

    #[test]
    fn test_union() {
        let a: GrowingBitGrid = "101\n011\n000".parse().unwrap();
        let b: GrowingBitGrid = "001\n101\n010".parse().unwrap();
        let c: GrowingBitGrid = "101\n111\n010".parse().unwrap();
        assert_eq!(a.union(&b).unwrap(), c);
    }

    #[test]
    fn test_intersection() {
        let a: GrowingBitGrid = "101\n011\n000".parse().unwrap();
        let b: GrowingBitGrid = "001\n101\n010".parse().unwrap();
        let c: GrowingBitGrid = "001\n001\n000".parse().unwrap();
        assert_eq!(a.intersection(&b).unwrap(), c);
    }
}
