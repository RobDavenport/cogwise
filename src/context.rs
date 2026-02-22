use rand_core::RngCore;

use crate::blackboard::Blackboard;

pub struct Context<'a> {
    tick: u64,
    delta_ticks: u32,
    blackboard: &'a mut Blackboard,
    rng: Option<&'a mut dyn RngCore>,
}

impl<'a> Context<'a> {
    pub fn new(
        tick: u64,
        delta_ticks: u32,
        blackboard: &'a mut Blackboard,
        rng: Option<&'a mut dyn RngCore>,
    ) -> Self {
        Self {
            tick,
            delta_ticks,
            blackboard,
            rng,
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn delta_ticks(&self) -> u32 {
        self.delta_ticks
    }

    pub fn blackboard(&self) -> &Blackboard {
        self.blackboard
    }

    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        self.blackboard
    }

    pub fn rng(&mut self) -> &mut dyn RngCore {
        self.rng
            .as_deref_mut()
            .expect("RNG required for selector method")
    }

    pub fn has_rng(&self) -> bool {
        self.rng.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Context;
    use crate::blackboard::Blackboard;
    use rand_core::{Error, RngCore};

    struct CountingRng(u32);
    impl RngCore for CountingRng {
        fn next_u32(&mut self) -> u32 {
            let out = self.0;
            self.0 = self.0.wrapping_add(1);
            out
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

    #[test]
    fn context_tick_count() {
        let mut bb = Blackboard::new();
        let ctx = Context::new(10, 2, &mut bb, None);
        assert_eq!(ctx.tick(), 10);
        assert_eq!(ctx.delta_ticks(), 2);
    }

    #[test]
    fn context_blackboard_read_write() {
        let mut bb = Blackboard::new();
        let mut ctx = Context::new(0, 1, &mut bb, None);
        ctx.blackboard_mut().set_int(1, 7);
        assert_eq!(ctx.blackboard().get_int(1), Some(7));
    }

    #[test]
    fn context_rng_access() {
        let mut bb = Blackboard::new();
        let mut rng = CountingRng(5);
        let mut ctx = Context::new(0, 1, &mut bb, Some(&mut rng));
        assert!(ctx.has_rng());
        assert_eq!(ctx.rng().next_u32(), 5);
    }
}
