pub mod angle;
pub mod point;
pub mod pose;

use bits::BitArray;
use num_traits::{Num, cast::ToPrimitive};
use std::{
    fmt::Display,
    iter::Sum,
    ops::{AddAssign, BitAnd, BitOr, BitXor, DivAssign, MulAssign, SubAssign},
    str::FromStr,
};
use trait_set::trait_set;

trait_set! {
    pub trait NumType = FromStr + Display + ToPrimitive + Default + Num + Copy + AddAssign + SubAssign + MulAssign + DivAssign + PartialOrd + Sum;
}

use crate::point::{GridPoint, Point};

fn span<N: NumType>(min: N, max: N) -> N {
    N::one() + max - min
}

fn height_width(s: &str) -> (usize, usize) {
    (
        s.trim().lines().count(),
        s.trim().lines().next().unwrap().trim().chars().count(),
    )
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BitGrid {
    bits: BitArray,
    min: GridPoint,
    max: GridPoint,
}

impl Default for BitGrid {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl FromIterator<(GridPoint, bool)> for BitGrid {
    fn from_iter<T: IntoIterator<Item = (GridPoint, bool)>>(iter: T) -> Self {
        let mut points = vec![];
        for v in iter {
            points.push(v);
        }
        let mut result = Self::setup(&points);
        for (p, value) in points {
            result.set(p, value);
        }
        result
    }
}

impl FromStr for BitGrid {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (height, width) = height_width(s);
        let mut result = Self::new(0, width as i64 - 1, 0, height as i64 - 1);
        result.destringify(|n| n as i64, s)?;
        Ok(result)
    }
}

impl Display for BitGrid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stringify())
    }
}

impl BitGrid {
    pub fn new(min_x: i64, max_x: i64, min_y: i64, max_y: i64) -> Self {
        let num_zeros = span(min_x, max_x) * span(min_y, max_y);
        Self {
            min: pt!(min_x, min_y),
            max: pt!(max_x, max_y),
            bits: BitArray::zeros(num_zeros as u64),
        }
    }

    pub fn translated(&self, translation: GridPoint) -> Self {
        Self {
            bits: self.bits.clone(),
            min: self.min + translation,
            max: self.max + translation,
        }
    }

    fn setup(points: &Vec<(GridPoint, bool)>) -> Self {
        let min_x = points.iter().map(|v| v.0[0]).min().unwrap();
        let max_x = points.iter().map(|v| v.0[0]).max().unwrap();
        let min_y = points.iter().map(|v| v.0[1]).min().unwrap();
        let max_y = points.iter().map(|v| v.0[1]).max().unwrap();
        Self::new(min_x, max_x, min_y, max_y)
    }

    pub fn num_bits(&self) -> u64 {
        self.width() as u64 * self.height() as u64
    }

    pub fn min_max_stored(&self) -> (GridPoint, GridPoint) {
        (self.min, self.max)
    }

    pub fn manhattan_neighbors(&self, p: &GridPoint) -> impl Iterator<Item = (GridPoint, bool)> {
        manhattan_iter(p).map(|p| (p, self.get(&p)))
    }

    fn bits(&self) -> &BitArray {
        &self.bits
    }

    pub fn get(&self, p: &GridPoint) -> bool {
        self.index_1d(p).map_or(false, |i| self.bits.is_set(i))
    }

    pub fn set(&mut self, p: GridPoint, value: bool) {
        match self.index_1d(&p) {
            Some(i) => {
                self.bits.set(i, value);
            }
            None => {
                let min = self.min.element_min(&p);
                let max = self.max.element_max(&p);
                self.resize(min, max);
                self.bits.set(self.unchecked_index_1d(&p), value);
            }
        }
    }

    pub fn width(&self) -> i64 {
        span(self.min[0], self.max[0])
    }

    pub fn height(&self) -> i64 {
        span(self.min[1], self.max[1])
    }

    pub fn coord_iter(&self) -> CoordIter {
        CoordIter {
            max_y: self.max[1],
            min_x: self.min[0],
            max_x: self.max[0],
            x: self.min[0],
            y: self.min[1],
        }
    }

    fn with_bits(&self, alt_bits: BitArray) -> Self {
        assert_eq!(alt_bits.len(), self.num_bits());
        Self {
            min: self.min,
            max: self.max,
            bits: alt_bits,
        }
    }

    pub fn words_used(&self) -> u64 {
        let base = self.bits().len() / 64;
        let extra = if self.bits().len() % 64 > 0 { 1 } else { 0 };
        base + extra
    }

    fn in_bounds(&self, p: &GridPoint) -> bool {
        self.min[0] <= p[0] && p[0] <= self.max[0] && self.min[1] <= p[1] && p[1] <= self.max[1]
    }

    pub fn iter(&self) -> impl Iterator<Item = (GridPoint, bool)> {
        self.coord_iter().map(|p| (p, self.get(&p)))
    }

    pub fn ones(&self) -> impl Iterator<Item = GridPoint> {
        self.bits.one_indices().map(|i| self.index_2d(i))
    }

    pub fn ones_touching_zeros(&self) -> impl Iterator<Item = GridPoint> {
        self.ones().filter(|p| {
            self.manhattan_neighbors(p)
                .filter(|(_, value)| *value)
                .count()
                < 4
        })
    }

    pub fn count_ones(&self) -> u64 {
        self.bits().count_ones()
    }

    fn stringify(&self) -> String {
        let mut s = String::new();
        for (p, value) in self.iter() {
            if p[1] > self.min[1] && p[0] == self.min[0] {
                s.push('\n');
            }
            let c = if value { '1' } else { '0' };
            s.push(c);
        }
        s
    }

    fn destringify<F: Fn(usize) -> i64>(&mut self, indexer: F, s: &str) -> anyhow::Result<()> {
        for (y, row) in s.trim().lines().enumerate() {
            for (x, cell) in row.trim().char_indices() {
                let value = match cell {
                    '1' | 'X' | '*' => true,
                    '0' | 'O' | '.' => false,
                    _ => return Err(anyhow::anyhow!("Illegal char: {cell}")),
                };
                self.set(pt!(indexer(x), indexer(y)), value);
            }
        }
        Ok(())
    }

    pub fn zero_clone(&self) -> Self {
        self.with_bits(BitArray::zeros(self.num_bits()))
    }

    pub fn overlapping_counts(&self, other: &Self) -> u64 {
        (self & other).count_ones()
    }

    fn resize(&mut self, min: GridPoint, max: GridPoint) {
        if min != self.min || max != self.max {
            let mut new_self = Self::new(min[0], max[0], min[1], max[1]);
            for (p, value) in self.iter() {
                assert!(new_self.in_bounds(&p));
                new_self.bits.set(new_self.unchecked_index_1d(&p), value);
            }
            std::mem::swap(&mut new_self, self);
        }
    }

    fn index_1d(&self, p: &GridPoint) -> Option<u64> {
        if self.in_bounds(p) {
            Some(self.unchecked_index_1d(p))
        } else {
            None
        }
    }

    fn unchecked_index_1d(&self, p: &GridPoint) -> u64 {
        let grid_x = p[0] - self.min[0];
        let grid_y = p[1] - self.min[1];
        (grid_y * self.width() + grid_x) as u64
    }

    fn index_2d(&self, i: u64) -> GridPoint {
        let i = i as i64;
        let uy = i / self.width();
        let ux = i % self.width();
        pt!(ux + self.min[0], uy + self.min[1])
    }
}

impl BitAnd for &BitGrid {
    type Output = BitGrid;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.ones().filter(|p| rhs.get(p)).collect()
    }
}

impl BitOr for &BitGrid {
    type Output = BitGrid;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut union = self.ones().collect::<BitGrid>();
        for one in rhs.ones() {
            union.set(one, true);
        }
        union
    }
}

impl BitXor for &BitGrid {
    type Output = BitGrid;

    fn bitxor(self, rhs: Self) -> Self::Output {
        let mut union = self.ones().collect::<BitGrid>();
        for one in rhs.ones() {
            union.set(one, !self.get(&one));
        }
        union
    }
}

impl FromIterator<GridPoint> for BitGrid {
    fn from_iter<T: IntoIterator<Item = GridPoint>>(iter: T) -> Self {
        let mut result = BitGrid::default();
        for p in iter {
            result.set(p, true);
        }
        result
    }
}

impl<'a> FromIterator<&'a GridPoint> for BitGrid {
    fn from_iter<T: IntoIterator<Item = &'a GridPoint>>(iter: T) -> Self {
        let mut result = BitGrid::default();
        for p in iter {
            result.set(*p, true);
        }
        result
    }
}

pub struct CoordIter {
    max_y: i64,
    min_x: i64,
    max_x: i64,
    x: i64,
    y: i64,
}

impl Iterator for CoordIter {
    type Item = GridPoint;

    fn next(&mut self) -> Option<Self::Item> {
        if self.y > self.max_y {
            None
        } else {
            let result = Some(pt!(self.x, self.y));
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

fn manhattan_iter(p: &GridPoint) -> impl Iterator<Item = GridPoint> {
    MANHATTAN_OFFSETS
        .iter()
        .copied()
        .map(move |(off_x, off_y)| pt!(off_x + p[0], off_y + p[1]))
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};

    use super::*;

    #[test]
    fn test_from_str() {
        let grid_str = "1101000\n1011000\n0010000\n";
        let grid = grid_str.parse::<BitGrid>().unwrap();
        assert_eq!(format!("{grid}\n"), grid_str);
        assert_eq!(grid.height(), 3);
        assert_eq!(grid.width(), 7);
        assert_eq!(grid.count_ones(), 7);
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
            assert_eq!(grid.get(&pt!(x, y)), value);
            assert!(grid.in_bounds(&pt!(x, y)));
        }

        // out of bounds - still false
        for (x, y) in [(-1, 0), (3, 3), (1, 3), (1, -3)] {
            assert_eq!(grid.get(&pt!(x, y)), false);
            assert!(!grid.in_bounds(&pt!(x, y)));
        }

        let zero_grid = grid.zero_clone();
        assert_eq!(zero_grid.min, grid.min);
        assert_eq!(zero_grid.max, grid.max);
        assert_eq!(zero_grid.width(), grid.width());
        assert_eq!(zero_grid.height(), grid.height());
        assert!(zero_grid.iter().all(|(_, value)| !value));
    }

    #[test]
    fn test_from_iter() {
        let pvs = [(pt!(2, 2), true), (pt!(-1, 3), false), (pt!(3, -2), true)];
        let test_points = pvs.iter().copied().collect::<HashMap<_, _>>();
        let grid = pvs.iter().copied().collect::<BitGrid>();
        for p in grid.ones() {
            assert!(test_points.get(&p).unwrap_or(&false));
        }
        assert_eq!(grid.count_ones(), 2);
        assert_eq!(grid.count_ones(), grid.ones().count() as u64);
        assert_eq!(grid.width(), 5);
        assert_eq!(grid.height(), 6);

        let ones1 = grid
            .iter()
            .filter(|(_, v)| *v)
            .map(|(p, _)| p)
            .collect::<Vec<_>>();
        let ones2 = grid.ones().collect::<Vec<_>>();
        for one in ones2.iter() {
            assert!(grid.get(one));
        }
        assert_eq!(ones1, ones2);
        for (p, value) in grid.iter() {
            println!("p: {p} {value}");
            match test_points.get(&p) {
                Some(expected) => {
                    assert_eq!(value, *expected);
                }
                None => {
                    assert_eq!(value, false);
                }
            }
        }

        let zeros = grid.zero_clone();
        assert_eq!(zeros.min, grid.min);
        assert_eq!(zeros.max, grid.max);
    }

    #[test]
    fn test_from_coord_iter() {
        let coords = vec![pt!(-2, -1), pt!(1, -1), pt!(-1, 1), pt!(1, 2), pt!(3, 4)];
        let coord_set = coords.iter().collect::<BitGrid>();
        let rebuilt = coord_set.ones().collect::<Vec<_>>();
        assert_eq!(coords, rebuilt);
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
            let one = one.parse::<BitGrid>().unwrap();
            let two = two.parse::<BitGrid>().unwrap();
            assert_eq!(count, one.overlapping_counts(&two));
        }
    }

    #[test]
    fn test_manhattan_iter() {
        let test_grid = "010\n100\n101".parse::<BitGrid>().unwrap();
        for (p, neighbors) in [
            (
                pt!(0, 0),
                vec![
                    (pt!(-1, 0), false),
                    (pt!(0, -1), false),
                    (pt!(1, 0), true),
                    (pt!(0, 1), true),
                ],
            ),
            (
                pt!(1, 1),
                vec![
                    (pt!(0, 1), true),
                    (pt!(1, 0), true),
                    (pt!(2, 1), false),
                    (pt!(1, 2), false),
                ],
            ),
            (
                pt!(1, 2),
                vec![
                    (pt!(0, 2), true),
                    (pt!(1, 1), false),
                    (pt!(2, 2), true),
                    (pt!(1, 3), false),
                ],
            ),
        ] {
            let actual = test_grid.manhattan_neighbors(&p).collect::<Vec<_>>();
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
        .parse::<BitGrid>()
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
        .map(|(x, y)| pt!(*x, *y))
        .collect::<BTreeSet<_>>();
        assert_eq!(expected, found);
        let one_count = test_grid.count_ones();
        assert_eq!(found.len() as u64 + 1, one_count);
        assert_eq!(51, test_grid.bits().len());
        assert_eq!(1, test_grid.words_used());
    }

    #[test]
    fn test_bit_or() {
        let a: BitGrid = "101\n011\n000".parse().unwrap();
        let b: BitGrid = "001\n101\n010".parse().unwrap();
        let c: BitGrid = "101\n111\n010".parse().unwrap();
        assert_eq!((&a | &b), c);
    }

    #[test]
    fn test_bit_and() {
        let a: BitGrid = "101\n011\n000".parse().unwrap();
        let b: BitGrid = "001\n101\n010".parse().unwrap();
        let c: BitGrid = [pt!(2, 0), pt!(2, 1)].iter().collect();
        assert_eq!((&a & &b), c);
    }

    #[test]
    fn test_bit_xor() {
        let a: BitGrid = "101\n011\n000".parse().unwrap();
        let b: BitGrid = "001\n101\n010".parse().unwrap();
        let c: BitGrid = "100\n110\n010".parse().unwrap();
        assert_eq!((&a ^ &b), c);
    }

    #[test]
    fn test_resize() {
        let mut a: BitGrid = "101\n011\n000".parse().unwrap();
        assert_eq!(9, a.bits().len());
        assert_eq!(1, a.words_used());
        a.set(pt!(-2, -2), true);
        assert_eq!(25, a.bits.len());
        assert_eq!(1, a.words_used());
        let ex1 = "10000\n00000\n00101\n00011\n00000";
        assert_eq!(ex1, format!("{a}").as_str());
        assert_eq!(a.min, pt!(-2, -2));
        assert_eq!(a.max, pt!(2, 2));
        a.set(pt!(-2, 3), true);

        let ex2 = "10000\n00000\n00101\n00011\n00000\n10000";
        assert_eq!(ex2, format!("{a}").as_str());
        assert_eq!(a.min, pt!(-2, -2));
        assert_eq!(a.max, pt!(2, 3));
        assert_eq!(30, a.bits.len());
        assert_eq!(1, a.words_used());
    }

    #[test]
    fn test_translation() {
        let a: BitGrid = "010\n111\n010".parse().unwrap();
        let zeroed = a.translated(pt!(-1, -1));
        let expected_ones = vec![pt!(0, -1), pt!(-1, 0), pt!(0, 0), pt!(1, 0), pt!(0, 1)];
        let zeroed_ones = zeroed.ones().collect::<Vec<_>>();
        assert_eq!(expected_ones, zeroed_ones);
    }
}
