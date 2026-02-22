use alloc::vec::Vec;

use crate::float::Float;

#[derive(Clone, Debug, PartialEq)]
pub enum ResponseCurve<F: Float> {
    Linear { slope: F, offset: F },
    Polynomial { exponent: F, offset: F },
    Logistic { midpoint: F, steepness: F },
    Step { threshold: F },
    Inverse { offset: F },
    Constant(F),
    CustomPoints(Vec<(F, F)>),
}

impl<F: Float> ResponseCurve<F> {
    pub fn evaluate(&self, x: F) -> F {
        let x = x.clamp(F::zero(), F::one());
        let raw = match self {
            ResponseCurve::Linear { slope, offset } => *slope * x + *offset,
            ResponseCurve::Polynomial { exponent, offset } => {
                (x + *offset).max(F::zero()).powf(*exponent)
            }
            ResponseCurve::Logistic {
                midpoint,
                steepness,
            } => {
                let exp_val = (F::zero() - *steepness * (x - *midpoint)).exp();
                F::one() / (F::one() + exp_val)
            }
            ResponseCurve::Step { threshold } => {
                if x >= *threshold {
                    F::one()
                } else {
                    F::zero()
                }
            }
            ResponseCurve::Inverse { offset } => {
                let denom = x + *offset;
                if denom <= F::zero() {
                    F::one()
                } else {
                    F::one() / denom
                }
            }
            ResponseCurve::Constant(v) => *v,
            ResponseCurve::CustomPoints(points) => piecewise_lerp(points, x),
        };

        raw.clamp(F::zero(), F::one())
    }
}

fn piecewise_lerp<F: Float>(points: &[(F, F)], x: F) -> F {
    if points.is_empty() {
        return F::zero();
    }
    if points.len() == 1 {
        return points[0].1;
    }

    if x <= points[0].0 {
        return points[0].1;
    }
    let last = points.len() - 1;
    if x >= points[last].0 {
        return points[last].1;
    }

    for window in points.windows(2) {
        let (x0, y0) = window[0];
        let (x1, y1) = window[1];
        if x >= x0 && x <= x1 {
            let span = x1 - x0;
            if span <= F::zero() {
                return y1;
            }
            let t = (x - x0) / span;
            return y0.lerp(y1, t);
        }
    }

    points[last].1
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::ResponseCurve;

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-3, "{left} != {right}");
    }

    #[test]
    fn curve_linear_identity() {
        let curve = ResponseCurve::Linear {
            slope: 1.0,
            offset: 0.0,
        };
        approx_eq(curve.evaluate(0.5), 0.5);
    }

    #[test]
    fn curve_linear_inverted() {
        let curve = ResponseCurve::Linear {
            slope: -1.0,
            offset: 1.0,
        };
        approx_eq(curve.evaluate(0.0), 1.0);
        approx_eq(curve.evaluate(1.0), 0.0);
    }

    #[test]
    fn curve_polynomial_quadratic() {
        let curve = ResponseCurve::Polynomial {
            exponent: 2.0,
            offset: 0.0,
        };
        approx_eq(curve.evaluate(0.5), 0.25);
    }

    #[test]
    fn curve_polynomial_sqrt() {
        let curve = ResponseCurve::Polynomial {
            exponent: 0.5,
            offset: 0.0,
        };
        approx_eq(curve.evaluate(0.25), 0.5);
    }

    #[test]
    fn curve_logistic_midpoint() {
        let curve = ResponseCurve::Logistic {
            midpoint: 0.5,
            steepness: 10.0,
        };
        approx_eq(curve.evaluate(0.5), 0.5);
    }

    #[test]
    fn curve_step_below() {
        let curve = ResponseCurve::Step { threshold: 0.7 };
        approx_eq(curve.evaluate(0.69), 0.0);
    }

    #[test]
    fn curve_step_above() {
        let curve = ResponseCurve::Step { threshold: 0.7 };
        approx_eq(curve.evaluate(0.7), 1.0);
        approx_eq(curve.evaluate(0.9), 1.0);
    }

    #[test]
    fn curve_inverse() {
        let curve = ResponseCurve::Inverse { offset: 0.1 };
        let y0 = curve.evaluate(0.0);
        let y1 = curve.evaluate(1.0);
        assert!(y0 >= y1);
    }

    #[test]
    fn curve_constant() {
        let curve = ResponseCurve::Constant(0.42);
        approx_eq(curve.evaluate(0.0), 0.42);
        approx_eq(curve.evaluate(1.0), 0.42);
    }

    #[test]
    fn curve_custom_points() {
        let curve = ResponseCurve::CustomPoints(vec![(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)]);
        approx_eq(curve.evaluate(0.25), 0.5);
        approx_eq(curve.evaluate(0.75), 0.5);
    }

    #[test]
    fn curve_clamp_output() {
        let curve = ResponseCurve::Linear {
            slope: 2.0,
            offset: 0.5,
        };
        assert_eq!(curve.evaluate(1.0), 1.0);
    }
}
