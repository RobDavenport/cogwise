use alloc::vec::Vec;
use core::cmp::Ordering;

use rand_core::RngCore;

use crate::blackboard::Blackboard;
use crate::float::Float;
use crate::utility::action::UtilityAction;

#[derive(Clone, Debug, PartialEq)]
pub enum SelectionMethod {
    HighestScore,
    WeightedRandom,
    TopN(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reasoner<F: Float, A> {
    pub actions: Vec<UtilityAction<F, A>>,
    pub selection_method: SelectionMethod,
}

impl<F: Float, A> Reasoner<F, A> {
    pub fn select(
        &self,
        blackboard: &Blackboard,
        current_action: Option<usize>,
        rng: Option<&mut dyn RngCore>,
    ) -> usize {
        if self.actions.is_empty() {
            return 0;
        }

        let scores: Vec<F> = self
            .actions
            .iter()
            .enumerate()
            .map(|(i, action)| action.score(blackboard, current_action == Some(i)))
            .collect();

        match self.selection_method {
            SelectionMethod::HighestScore => scores
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
                .map(|(idx, _)| idx)
                .unwrap_or(0),
            SelectionMethod::WeightedRandom => {
                let rng = rng.expect("WeightedRandom requires RNG");
                let mut positive = Vec::with_capacity(scores.len());
                let mut total = F::zero();
                for score in &scores {
                    let val = if *score > F::zero() {
                        *score
                    } else {
                        F::zero()
                    };
                    positive.push(val);
                    total = total + val;
                }
                if total <= F::zero() {
                    return 0;
                }

                let roll_01 = (rng.next_u32() as f32) / ((u32::MAX as f32) + 1.0);
                let roll = F::from_f32(roll_01) * total;
                let mut cumulative = F::zero();
                for (i, score) in positive.iter().enumerate() {
                    cumulative = cumulative + *score;
                    if roll < cumulative {
                        return i;
                    }
                }

                positive.len() - 1
            }
            SelectionMethod::TopN(n) => {
                let rng = rng.expect("TopN requires RNG");
                let mut indices: Vec<usize> = (0..scores.len()).collect();
                indices.sort_by(|&a, &b| scores[b].partial_cmp(&scores[a]).unwrap_or(Ordering::Equal));
                let n = n.max(1).min(indices.len());
                indices[rng.next_u32() as usize % n]
            }
        }
    }

    pub fn score_all(
        &self,
        blackboard: &Blackboard,
        current_action: Option<usize>,
    ) -> Vec<(usize, F)> {
        let mut out: Vec<(usize, F)> = self
            .actions
            .iter()
            .enumerate()
            .map(|(i, action)| (i, action.score(blackboard, current_action == Some(i))))
            .collect();

        out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        out
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use crate::blackboard::Blackboard;
    use crate::utility::action::UtilityAction;
    use crate::utility::consideration::Consideration;
    use crate::utility::curve::ResponseCurve;
    use crate::utility::reasoner::{Reasoner, SelectionMethod};
    use rand_core::{Error, RngCore};

    struct SeqRng {
        values: Vec<u32>,
        idx: usize,
    }

    impl SeqRng {
        fn new(values: Vec<u32>) -> Self {
            Self { values, idx: 0 }
        }
    }

    impl RngCore for SeqRng {
        fn next_u32(&mut self) -> u32 {
            let value = self.values[self.idx % self.values.len()];
            self.idx += 1;
            value
        }

        fn next_u64(&mut self) -> u64 {
            self.next_u32() as u64
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for chunk in dest.chunks_mut(4) {
                let n = self.next_u32().to_le_bytes();
                let len = chunk.len();
                chunk.copy_from_slice(&n[..len]);
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    fn linear(input_key: u32) -> Consideration<f32> {
        Consideration {
            input_key,
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
    fn reasoner_highest_score() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.2);
        bb.set_float(2, 0.8);
        let reasoner = Reasoner {
            actions: vec![
                UtilityAction {
                    action_id: 10u32,
                    considerations: vec![linear(1)],
                    weight: 1.0,
                    momentum: 0.0,
                },
                UtilityAction {
                    action_id: 20u32,
                    considerations: vec![linear(2)],
                    weight: 1.0,
                    momentum: 0.0,
                },
            ],
            selection_method: SelectionMethod::HighestScore,
        };
        assert_eq!(reasoner.select(&bb, None, None), 1);
    }

    #[test]
    fn reasoner_top_n() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.1);
        bb.set_float(2, 0.5);
        bb.set_float(3, 0.9);
        let reasoner = Reasoner {
            actions: vec![
                UtilityAction {
                    action_id: 1u32,
                    considerations: vec![linear(1)],
                    weight: 1.0,
                    momentum: 0.0,
                },
                UtilityAction {
                    action_id: 2u32,
                    considerations: vec![linear(2)],
                    weight: 1.0,
                    momentum: 0.0,
                },
                UtilityAction {
                    action_id: 3u32,
                    considerations: vec![linear(3)],
                    weight: 1.0,
                    momentum: 0.0,
                },
            ],
            selection_method: SelectionMethod::TopN(2),
        };

        let mut rng = SeqRng::new(vec![1]);
        let idx = reasoner.select(&bb, None, Some(&mut rng));
        assert!(idx == 1 || idx == 2);
    }

    #[test]
    fn reasoner_weighted_random_distribution() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 0.1);
        bb.set_float(2, 0.9);
        let reasoner = Reasoner {
            actions: vec![
                UtilityAction {
                    action_id: 1u32,
                    considerations: vec![linear(1)],
                    weight: 1.0,
                    momentum: 0.0,
                },
                UtilityAction {
                    action_id: 2u32,
                    considerations: vec![linear(2)],
                    weight: 1.0,
                    momentum: 0.0,
                },
            ],
            selection_method: SelectionMethod::WeightedRandom,
        };

        let mut rng = SeqRng::new((0..500).map(|i| i * 8_589_934).collect());
        let mut high = 0usize;
        let mut low = 0usize;
        for _ in 0..200 {
            let idx = reasoner.select(&bb, None, Some(&mut rng));
            if idx == 1 {
                high += 1;
            } else {
                low += 1;
            }
        }
        assert!(high > low, "expected high score selected more often");
    }
}
