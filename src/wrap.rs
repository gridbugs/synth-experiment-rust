use std::{
    marker::PhantomData,
    ops::{Add, AddAssign},
};

pub trait RangeF64 {
    const MIN: f64;
    const MAX: f64;
    const DELTA: f64 = Self::MAX - Self::MIN;
}

#[derive(Debug, Clone, Copy)]
pub struct RangeF64Unit;

impl RangeF64 for RangeF64Unit {
    const MIN: f64 = 0f64;
    const MAX: f64 = 1f64;
}

#[derive(Debug, Clone, Copy)]
pub struct RangeF64Radians;

impl RangeF64 for RangeF64Radians {
    const MIN: f64 = -std::f64::consts::PI;
    const MAX: f64 = std::f64::consts::PI;
}

#[derive(Debug)]
pub struct WrapF64<R: RangeF64> {
    value: f64,
    range: PhantomData<R>,
}

impl<R: RangeF64> Clone for WrapF64<R> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            range: PhantomData,
        }
    }
}

impl<R: RangeF64> Copy for WrapF64<R> {}

fn wrap_f64<R: RangeF64>(value: f64) -> f64 {
    (value - R::MIN).rem_euclid(R::DELTA) + R::MIN
}

impl<R: RangeF64> WrapF64<R> {
    pub fn new(value: f64) -> Self {
        Self {
            value: wrap_f64::<R>(value),
            range: PhantomData,
        }
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

pub type WrapF64Unit = WrapF64<RangeF64Unit>;
pub type WrapF64Radians = WrapF64<RangeF64Radians>;

impl<R: RangeF64> From<f64> for WrapF64<R> {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl<R: RangeF64> From<WrapF64<R>> for f64 {
    fn from(value: WrapF64<R>) -> Self {
        value.value()
    }
}

impl<R: RangeF64> Add for WrapF64<R> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.value() + rhs.value())
    }
}

impl<R: RangeF64> AddAssign for WrapF64<R> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<R: RangeF64> Add<f64> for WrapF64<R> {
    type Output = Self;
    fn add(self, rhs: f64) -> Self::Output {
        Self::new(self.value() + rhs)
    }
}

impl<R: RangeF64> AddAssign<f64> for WrapF64<R> {
    fn add_assign(&mut self, rhs: f64) {
        *self = *self + rhs;
    }
}

#[cfg(test)]
mod test {

    macro_rules! assert_approx_eq_f64 {
        ($lhs: expr, $rhs: expr) => {
            assert!(
                ($lhs - $rhs).abs() < std::f64::EPSILON,
                "abs({} - {}) >= {}",
                $lhs,
                $rhs,
                std::f64::EPSILON
            )
        };
    }

    use super::*;
    #[test]
    fn wrap() {
        assert_approx_eq_f64!(
            WrapF64Radians::new(4f64).value(),
            4f64 - (std::f64::consts::PI * 2f64)
        );
        assert_approx_eq_f64!(WrapF64Unit::new(-1.2).value(), 0.8f64);
    }
}
