#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decorator {
    Inverter,
    Repeat(u32),
    Retry(u32),
    Cooldown(u32),
    Guard(u32),
    UntilSuccess,
    UntilFail,
    Timeout(u32),
    ForceSuccess,
    ForceFailure,
}

#[cfg(test)]
mod tests {
    use super::Decorator;

    #[test]
    fn decorator_clone() {
        let all = [
            Decorator::Inverter,
            Decorator::Repeat(1),
            Decorator::Retry(2),
            Decorator::Cooldown(3),
            Decorator::Guard(4),
            Decorator::UntilSuccess,
            Decorator::UntilFail,
            Decorator::Timeout(5),
            Decorator::ForceSuccess,
            Decorator::ForceFailure,
        ];

        for d in all {
            let copy = d.clone();
            assert_eq!(copy, d);
            let _ = alloc::format!("{copy:?}");
        }
    }
}
