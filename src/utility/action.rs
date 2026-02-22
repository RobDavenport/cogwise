use alloc::vec::Vec;

use crate::blackboard::Blackboard;
use crate::float::Float;
use crate::utility::consideration::Consideration;

#[derive(Clone, Debug, PartialEq)]
pub struct UtilityAction<F: Float, A> {
    pub action_id: A,
    pub considerations: Vec<Consideration<F>>,
    pub weight: F,
    pub momentum: F,
}

impl<F: Float, A> UtilityAction<F, A> {
    pub fn score(&self, blackboard: &Blackboard, is_current: bool) -> F {
        if self.considerations.is_empty() {
            return self.weight;
        }

        let mut product = F::one();
        for consideration in &self.considerations {
            product = product * consideration.evaluate(blackboard);
        }

        let n = self.considerations.len();
        let inv_n = F::one() / F::from_f32(n as f32);
        let geo_mean = product.powf(inv_n);
        let mut score = geo_mean * self.weight;

        if is_current {
            score = score + self.momentum;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::blackboard::Blackboard;
    use crate::utility::action::UtilityAction;
    use crate::utility::consideration::Consideration;
    use crate::utility::curve::ResponseCurve;

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-3, "{left} != {right}");
    }

    fn linear_consideration(key: u32) -> Consideration<f32> {
        Consideration {
            input_key: key,
            curve: ResponseCurve::Linear {
                slope: 1.0,
                offset: 0.0,
            },
            weight: 1.0,
            input_min: 0.0,
            input_max: 1.0,
        }
    }

    #[test]
    fn utility_action_geometric_mean() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.5);
        bb.set_float(2, 0.5);
        let action = UtilityAction {
            action_id: 1u32,
            considerations: vec![linear_consideration(1), linear_consideration(2)],
            weight: 1.0,
            momentum: 0.0,
        };
        approx_eq(action.score(&bb, false), 0.5);
    }

    #[test]
    fn utility_action_zero_vetoes() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.9);
        bb.set_float(2, 0.0);
        let action = UtilityAction {
            action_id: 1u32,
            considerations: vec![linear_consideration(1), linear_consideration(2)],
            weight: 1.0,
            momentum: 0.0,
        };
        approx_eq(action.score(&bb, false), 0.0);
    }

    #[test]
    fn utility_action_momentum_bonus() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.4);
        let action = UtilityAction {
            action_id: 1u32,
            considerations: vec![linear_consideration(1)],
            weight: 1.0,
            momentum: 0.2,
        };
        approx_eq(action.score(&bb, true), 0.6);
    }

    #[test]
    fn utility_action_empty_considerations() {
        let bb = Blackboard::new();
        let action = UtilityAction::<f32, u32> {
            action_id: 1,
            considerations: vec![],
            weight: 0.7,
            momentum: 0.3,
        };
        approx_eq(action.score(&bb, false), 0.7);
    }
}
