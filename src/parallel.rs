#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParallelPolicy {
    RequireAll,
    RequireOne,
    RequireN(usize),
}

#[cfg(test)]
mod tests {
    use super::ParallelPolicy;

    #[test]
    fn parallel_policy_variants() {
        let a = ParallelPolicy::RequireAll;
        let b = ParallelPolicy::RequireOne;
        let c = ParallelPolicy::RequireN(2);
        assert!(matches!(a, ParallelPolicy::RequireAll));
        assert!(matches!(b, ParallelPolicy::RequireOne));
        assert!(matches!(c, ParallelPolicy::RequireN(2)));
    }
}
