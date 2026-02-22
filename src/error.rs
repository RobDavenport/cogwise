#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeError {
    EmptyComposite,
    MaxDepthExceeded(usize),
    WeightCountMismatch { children: usize, weights: usize },
    UtilityIdCountMismatch { children: usize, ids: usize },
    UnbalancedBuilder(usize),
}

#[cfg(test)]
mod tests {
    use super::TreeError;

    #[test]
    fn tree_error_variants() {
        let all = [
            TreeError::EmptyComposite,
            TreeError::MaxDepthExceeded(99),
            TreeError::WeightCountMismatch {
                children: 2,
                weights: 1,
            },
            TreeError::UtilityIdCountMismatch { children: 3, ids: 4 },
            TreeError::UnbalancedBuilder(1),
        ];

        assert!(matches!(all[0], TreeError::EmptyComposite));
    }
}
