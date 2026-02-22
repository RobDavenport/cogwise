pub trait Float:
    Copy
    + Clone
    + PartialEq
    + PartialOrd
    + core::ops::Add<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::Mul<Output = Self>
    + core::ops::Div<Output = Self>
    + core::ops::Neg<Output = Self>
    + Default
    + core::fmt::Debug
{
    fn zero() -> Self;
    fn one() -> Self;
    fn half() -> Self;
    fn two() -> Self;
    fn from_f32(v: f32) -> Self;
    fn to_f32(self) -> f32;
    fn sqrt(self) -> Self;
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn abs(self) -> Self;
    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;
    fn powf(self, exp: Self) -> Self;

    fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    fn lerp(self, other: Self, t: Self) -> Self {
        self + (other - self) * t
    }
}

impl Float for f32 {
    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }

    fn half() -> Self {
        0.5
    }

    fn two() -> Self {
        2.0
    }

    fn from_f32(v: f32) -> Self {
        v
    }

    fn to_f32(self) -> f32 {
        self
    }

    fn sqrt(self) -> Self {
        libm::sqrtf(self)
    }

    fn exp(self) -> Self {
        libm::expf(self)
    }

    fn ln(self) -> Self {
        libm::logf(self)
    }

    fn abs(self) -> Self {
        libm::fabsf(self)
    }

    fn min(self, other: Self) -> Self {
        if self < other { self } else { other }
    }

    fn max(self, other: Self) -> Self {
        if self > other { self } else { other }
    }

    fn powf(self, exp: Self) -> Self {
        libm::powf(self, exp)
    }
}

impl Float for f64 {
    fn zero() -> Self {
        0.0
    }

    fn one() -> Self {
        1.0
    }

    fn half() -> Self {
        0.5
    }

    fn two() -> Self {
        2.0
    }

    fn from_f32(v: f32) -> Self {
        v as f64
    }

    fn to_f32(self) -> f32 {
        self as f32
    }

    fn sqrt(self) -> Self {
        libm::sqrt(self)
    }

    fn exp(self) -> Self {
        libm::exp(self)
    }

    fn ln(self) -> Self {
        libm::log(self)
    }

    fn abs(self) -> Self {
        libm::fabs(self)
    }

    fn min(self, other: Self) -> Self {
        if self < other { self } else { other }
    }

    fn max(self, other: Self) -> Self {
        if self > other { self } else { other }
    }

    fn powf(self, exp: Self) -> Self {
        libm::pow(self, exp)
    }
}

#[cfg(test)]
mod tests {
    use super::Float;

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-4, "{left} != {right}");
    }

    #[test]
    fn float_f32_basics() {
        assert_eq!(f32::zero(), 0.0);
        assert_eq!(f32::one(), 1.0);
        assert_eq!(f32::from_f32(2.5), 2.5);
    }

    #[test]
    fn float_f32_math() {
        approx_eq(4.0f32.sqrt(), 2.0);
        approx_eq(0.0f32.exp(), 1.0);
        approx_eq(1.0f32.ln(), 0.0);
    }
}
