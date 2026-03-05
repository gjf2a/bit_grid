use enum_iterator::Sequence;

#[derive(Default, Copy, Clone)]
pub struct Diagonal {
    next_x: u64,
    next_y: u64,
    next_sum: u64,
}

impl Iterator for Diagonal {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        let result = Some((self.next_x, self.next_y));
        if self.next_x == 0 {
            self.next_sum += 1;
            self.next_x = self.next_sum;
            self.next_y = 0;
        } else {
            self.next_x -= 1;
            self.next_y += 1;
        }
        result
    }
}

#[derive(Sequence, Copy, Clone, Default)]
enum NegCycle {
    #[default]
    PosPos,
    PosNeg,
    NegPos,
    NegNeg,
}

impl NegCycle {
    fn neg(&self, value: (u64, u64)) -> (i64, i64) {
        match self {
            NegCycle::PosPos => (value.0 as i64, value.1 as i64),
            NegCycle::PosNeg => (value.0 as i64, -(value.1 as i64)),
            NegCycle::NegPos => (-(value.0 as i64), value.1 as i64),
            NegCycle::NegNeg => (-(value.0 as i64), -(value.1 as i64)),
        }
    }
}

#[derive(Copy, Clone, Default)]
pub struct Spiral {
    positives: Diagonal,
    positive: (u64, u64),
    neg: NegCycle,
    prev: (i64, i64)
}

impl Iterator for Spiral {
    type Item = (i64, i64);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.neg.neg(self.positive);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::spiral::Diagonal;

    #[test]
    fn explore() {
        let spiral = Diagonal::default();
        for (i, (x, y)) in spiral.take(20).enumerate() {
            println!("{i}: ({x}, {y})");
        }   
    }
}