#[derive(Clone, Debug)]
pub struct TreeConfig {
    pub max_depth: usize,
    pub max_ticks_per_frame: usize,
}

impl Default for TreeConfig {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_ticks_per_frame: 10_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TreeConfig;

    #[test]
    fn tree_config_defaults() {
        let cfg = TreeConfig::default();
        assert_eq!(cfg.max_depth, 64);
        assert_eq!(cfg.max_ticks_per_frame, 10_000);
    }
}
