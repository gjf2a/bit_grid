use std::{
    cmp::{max, min},
    fmt::Display,
    ops::{Add, AddAssign, Sub},
    str::FromStr,
};

use bits::BitArray;
use num_traits::{One, Zero};
use trait_set::trait_set;

trait_set! {
    pub trait BitGridIndex = Copy + Clone + Ord + PartialOrd + Eq + PartialEq + AddAssign + Zero + One + Add + Sub<Output = Self> + Display;
}

fn span<I: BitGridIndex>(min: I, max: I) -> I {
    I::one() + max - min
}

pub trait BitGrid {
    type Index: BitGridIndex;

    fn bits(&self) -> &BitArray;
    fn num_bits(&self) -> u64;
    fn with_bits(&self, alt_bits: BitArray) -> Self;
    fn matching_dimensions(&self, other: &Self) -> bool;
    fn is_set(&self, x: Self::Index, y: Self::Index) -> bool;
    fn set(&mut self, x: Self::Index, y: Self::Index, value: bool);

    fn min_x(&self) -> Self::Index;
    fn max_x(&self) -> Self::Index;
    fn min_y(&self) -> Self::Index;
    fn max_y(&self) -> Self::Index;
    fn coord_iter(&self) -> CoordIter<Self::Index>;

    fn in_bounds(&self, x: Self::Index, y: Self::Index) -> bool {
        self.min_x() <= x && x <= self.max_x() && self.min_y() <= y && y <= self.max_y()
    }

    fn manhattan_neighbors(
        &self,
        x: Self::Index,
        y: Self::Index,
    ) -> impl Iterator<Item = (Self::Index, Self::Index, bool)>;

    fn width(&self) -> Self::Index {
        span(self.min_x(), self.max_x())
    }

    fn height(&self) -> Self::Index {
        span(self.min_y(), self.max_y())
    }

    fn iter(&self) -> impl Iterator<Item = (Self::Index, Self::Index, bool)> {
        self.coord_iter().map(|(x, y)| (x, y, self.is_set(x, y)))
    }

    fn ones(&self) -> impl Iterator<Item = (Self::Index, Self::Index)> {
        self.coord_iter().filter(|(x, y)| self.is_set(*x, *y))
    }

    fn ones_touching_zeros(&self) -> impl Iterator<Item = (Self::Index, Self::Index)> {
        self.ones().filter(|(x, y)| {
            self.manhattan_neighbors(*x, *y)
                .filter(|(_, _, value)| *value)
                .count()
                < 4
        })
    }

    fn count_bits_on(&self) -> u64 {
        self.bits().count_bits_on()
    }

    fn stringify(&self) -> String {
        let mut s = String::new();
        for (x, y, value) in self.iter() {
            if y > self.min_y() && x == self.min_x() {
                s.push('\n');
            }
            let c = if value { '1' } else { '0' };
            s.push(c);
        }
        s
    }

    fn destringify<F: Fn(usize) -> Self::Index>(
        &mut self,
        indexer: F,
        s: &str,
    ) -> anyhow::Result<()> {
        for (y, row) in s.trim().lines().enumerate() {
            for (x, cell) in row.trim().char_indices() {
                let value = match cell {
                    '1' | 'X' | '*' => true,
                    '0' | 'O' | '.' => false,
                    _ => return Err(anyhow::anyhow!("Illegal char: {cell}")),
                };
                self.set(indexer(x), indexer(y), value);
            }
        }
        Ok(())
    }

    fn zero_clone(&self) -> Self where Self: Sized {
        self.with_bits(BitArray::zeros(self.num_bits()))
    }

    fn intersection(&self, other: &Self) -> Option<Self> where Self: Sized {
        if self.matching_dimensions(other) {
            Some(self.with_bits(self.bits() & other.bits()))
        } else {
            None
        }
    }

    fn union(&self, other: &Self) -> Option<Self> where Self: Sized {
        if self.matching_dimensions(other) {
            Some(self.with_bits(self.bits() | other.bits()))
        } else {
            None
        }
    }

    fn overlapping_counts(&self, other: &Self) -> Option<u64> where Self: Sized {
        self.intersection(other)
            .map(|overlaps| overlaps.count_bits_on())
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FixedBitGrid {
    bits: BitArray,
    width: u64,
    height: u64,
}

impl FixedBitGrid {
    pub fn new(width: u64, height: u64) -> Self {
        assert!(width >= 1 && height >= 1);
        Self {
            bits: BitArray::zeros(width * height),
            width,
            height,
        }
    }

    fn index_1d(&self, x: u64, y: u64) -> u64 {
        y * self.width() + x
    }
}

fn height_width(s: &str) -> (usize, usize) {
    (s.trim().lines().count(), s.trim().lines().next().unwrap().trim().chars().count())
}

impl FromStr for FixedBitGrid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (height, width) = height_width(s);
        let mut result = Self::new(width as u64, height as u64);
        result.destringify(|n| n as u64, s)?;
        Ok(result)
    }
}

impl Display for FixedBitGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stringify())
    }
}

impl BitGrid for FixedBitGrid {
    type Index = u64;

    fn num_bits(&self) -> u64 {
        self.width * self.height
    }

    fn with_bits(&self, alt_bits: BitArray) -> Self {
        assert_eq!(alt_bits.len(), self.bits.len());
        Self {
            bits: alt_bits,
            width: self.width,
            height: self.height,
        }
    }

    fn bits(&self) -> &BitArray {
        &self.bits
    }

    fn count_bits_on(&self) -> u64 {
        self.bits.count_bits_on()
    }

    fn manhattan_neighbors(
        &self,
        x: Self::Index,
        y: Self::Index,
    ) -> impl Iterator<Item = (Self::Index, Self::Index, bool)> {
        manhattan_iter(x as i64, y as i64)
            .filter(|(x, y)| *x >= 0 && *y >= 0)
            .map(|(x, y)| (x as u64, y as u64, self.is_set(x as u64, y as u64)))
    }

    fn matching_dimensions(&self, other: &Self) -> bool {
        self.width == other.width && self.height == other.height
    }

    fn is_set(&self, x: Self::Index, y: Self::Index) -> bool {
        if self.in_bounds(x, y) {
            self.bits.is_set(self.index_1d(x, y))
        } else {
            false
        }
    }

    fn set(&mut self, x: Self::Index, y: Self::Index, value: bool) {
        assert!(self.in_bounds(x, y));
        self.bits.set(self.index_1d(x, y), value);
    }

    fn coord_iter(&self) -> CoordIter<Self::Index> {
        CoordIter {
            max_y: self.height - 1,
            min_x: 0,
            max_x: self.width - 1,
            x: 0,
            y: 0,
        }
    }

    fn min_x(&self) -> Self::Index {
        0
    }

    fn max_x(&self) -> Self::Index {
        self.width - 1
    }

    fn min_y(&self) -> Self::Index {
        0
    }

    fn max_y(&self) -> Self::Index {
        self.height - 1
    }
}

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
        let mut points = vec![];
        for v in iter {
            points.push(v);
        }
        let mut result = Self::setup(&points);
        for (x, y, value) in points {
            result.set(x, y, value);
        }
        result
    }
}

impl FromStr for GrowingBitGrid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (height, width) = height_width(s);
        let mut result = Self::new(0, width as i64 - 1, 0, height as i64 - 1);
        result.destringify(|n| n as i64, s)?;
        Ok(result)
    }
}

impl Display for GrowingBitGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stringify())
    }
}

impl BitGrid for GrowingBitGrid {
    type Index = i64;

    fn num_bits(&self) -> u64 {
        self.width() as u64 * self.height() as u64
    }

    fn manhattan_neighbors(&self, x: i64, y: i64) -> impl Iterator<Item = (i64, i64, bool)> {
        manhattan_iter(x, y).map(|(x, y)| (x, y, self.is_set(x, y)))
    }

    fn bits(&self) -> &BitArray {
        &self.bits
    }

    fn is_set(&self, x: i64, y: i64) -> bool {
        self.index_1d(x, y).map_or(false, |i| self.bits.is_set(i))
    }

    fn set(&mut self, x: i64, y: i64, value: bool) {
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

    fn width(&self) -> i64 {
        span(self.min_x, self.max_x)
    }

    fn height(&self) -> i64 {
        span(self.min_y, self.max_y)
    }

    fn matching_dimensions(&self, other: &Self) -> bool {
        self.min_x == other.min_x
            && self.min_y == other.min_y
            && self.max_x == other.max_x
            && self.max_y == other.max_y
    }

    fn coord_iter(&self) -> CoordIter<Self::Index> {
        CoordIter {
            max_y: self.max_y,
            min_x: self.min_x,
            max_x: self.max_x,
            x: self.min_x,
            y: self.min_y,
        }
    }

    fn min_x(&self) -> Self::Index {
        self.min_x
    }

    fn max_x(&self) -> Self::Index {
        self.max_x
    }

    fn min_y(&self) -> Self::Index {
        self.min_y
    }

    fn max_y(&self) -> Self::Index {
        self.max_y
    }

    fn with_bits(&self, alt_bits: BitArray) -> Self {
        assert_eq!(alt_bits.len(), self.num_bits());
        Self {
            min_x: self.min_x,
            max_x: self.max_x,
            min_y: self.min_y,
            max_y: self.max_y,
            bits: alt_bits,
        }
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

    fn setup(points: &Vec<(i64, i64, bool)>) -> Self {
        let min_x = points.iter().map(|v| v.0).min().unwrap();
        let max_x = points.iter().map(|v| v.0).max().unwrap();
        let min_y = points.iter().map(|v| v.1).min().unwrap();
        let max_y = points.iter().map(|v| v.1).max().unwrap();
        Self::new(min_x, max_x, min_y, max_y)
    }

    pub fn downsize_to(&mut self, reference: &Self) {
        self.resize(reference.min_x, reference.max_x, reference.min_y, reference.max_y);
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

    fn index_1d(&self, x: i64, y: i64) -> Option<u64> {
        if self.in_bounds(x, y) {
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

pub struct CoordIter<I: BitGridIndex> {
    max_y: I,
    min_x: I,
    max_x: I,
    x: I,
    y: I,
}

impl<I: BitGridIndex> Iterator for CoordIter<I> {
    type Item = (I, I);

    fn next(&mut self) -> Option<Self::Item> {
        if self.y > self.max_y {
            None
        } else {
            let result = Some((self.x, self.y));
            self.x += I::one();
            if self.x > self.max_x {
                self.x = self.min_x;
                self.y += I::one();
            }
            result
        }
    }
}

const MANHATTAN_OFFSETS: [(i64, i64); 4] = [(-1, 0), (0, -1), (1, 0), (0, 1)];

fn manhattan_iter(x: i64, y: i64) -> impl Iterator<Item = (i64, i64)> {
    MANHATTAN_OFFSETS
        .iter()
        .copied()
        .map(move |(off_x, off_y)| (off_x + x, off_y + y))
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
        assert_eq!(grid.width(), 5);
        assert_eq!(grid.height(), 6);
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

    #[test]
    fn test_resize() {
        let mut a: GrowingBitGrid = "101\n011\n000".parse().unwrap();
        a.set(-2, -2, true);
        let ex1 = "00000000
00000000
00000000
00010000
00000000
00000101
00000011
00000000";
        assert_eq!(ex1, format!("{a}").as_str());
        assert_eq!(a.x_min_x_max_y_min_y_max(), (-5, 2, -5, 2));
        a.set(-2, 3, true);

        let ex2 = "00000000
00000000
00000000
00010000
00000000
00000101
00000011
00000000
00010000
00000000
00000000
00000000
00000000
00000000
00000000
00000000
00000000";
        assert_eq!(ex2, format!("{a}").as_str());
        assert_eq!(a.x_min_x_max_y_min_y_max(), (-5, 2, -5, 11));
    }
}
