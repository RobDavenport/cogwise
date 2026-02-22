use crate::blackboard::Blackboard;
use crate::float::Float;
use crate::utility::curve::ResponseCurve;

#[derive(Clone, Debug, PartialEq)]
pub struct Consideration<F: Float> {
    pub input_key: u32,
    pub curve: ResponseCurve<F>,
    pub weight: F,
    pub input_min: F,
    pub input_max: F,
}

impl<F: Float> Consideration<F> {
    pub fn evaluate(&self, blackboard: &Blackboard) -> F {
        let raw = match blackboard.get(self.input_key) {
            Some(value) => F::from_f32(value.to_score_f32()),
            None => return F::zero(),
        };

        let range = self.input_max - self.input_min;
        let normalized = if range.abs() <= F::from_f32(1.0e-6) {
            F::zero()
        } else {
            (raw - self.input_min) / range
        }
        .clamp(F::zero(), F::one());

        self.curve.evaluate(normalized) * self.weight
    }
}

#[cfg(test)]
mod tests {
    use crate::blackboard::Blackboard;
    use crate::utility::consideration::Consideration;
    use crate::utility::curve::ResponseCurve;

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-4, "{left} != {right}");
    }

    #[test]
    fn consideration_reads_blackboard() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.5);
        let c = Consideration {
            input_key: 1,
            curve: ResponseCurve::Linear {
                slope: 1.0,
                offset: 0.0,
            },
            weight: 1.0,
            input_min: 0.0,
            input_max: 1.0,
        };
        approx_eq(c.evaluate(&bb), 0.5);
    }

    #[test]
    fn consideration_missing_key() {
        let bb = Blackboard::new();
        let c = Consideration {
            input_key: 99,
            curve: ResponseCurve::Constant(1.0),
            weight: 1.0,
            input_min: 0.0,
            input_max: 1.0,
        };
        approx_eq(c.evaluate(&bb), 0.0);
    }

    #[test]
    fn consideration_missing_key_returns_zero() {
        let bb = Blackboard::new();
        let c = Consideration {
            input_key: 88,
            curve: ResponseCurve::Linear {
                slope: 1.0,
                offset: 0.0,
            },
            weight: 1.0,
            input_min: 0.0,
            input_max: 100.0,
        };
        approx_eq(c.evaluate(&bb), 0.0);
    }

    #[test]
    fn consideration_normalizes_input() {
        let mut bb = Blackboard::new();
        bb.set_float(3, 50.0);
        let c = Consideration {
            input_key: 3,
            curve: ResponseCurve::Linear {
                slope: 1.0,
                offset: 0.0,
            },
            weight: 1.0,
            input_min: 0.0,
            input_max: 100.0,
        };
        approx_eq(c.evaluate(&bb), 0.5);
    }
}
