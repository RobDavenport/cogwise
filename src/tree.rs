use alloc::vec;
use alloc::vec::Vec;

use rand_core::RngCore;

use crate::tick::{assign_ids, tick_node, NodeState};
use crate::{
    ActionHandler, BehaviorNode, Blackboard, ConditionHandler, Context, Observer, Status,
};

pub struct BehaviorTree<A, C> {
    root: BehaviorNode<A, C>,
    states: Vec<NodeState>,
    blackboard: Blackboard,
    tick_count: u64,
}

impl<A, C> BehaviorTree<A, C> {
    pub fn new(root: BehaviorNode<A, C>) -> Self {
        let node_count = assign_ids(&root).max(1);
        Self {
            root,
            states: vec![NodeState::default(); node_count],
            blackboard: Blackboard::new(),
            tick_count: 0,
        }
    }

    pub fn tick<AH, CH, O>(
        &mut self,
        action_handler: &mut AH,
        condition_handler: &CH,
        observer: &mut O,
    ) -> Status
    where
        AH: ActionHandler<A>,
        CH: ConditionHandler<C>,
        O: Observer,
    {
        self.tick_with(1, None, action_handler, condition_handler, observer)
    }

    pub fn tick_with<'a, AH, CH, O>(
        &'a mut self,
        delta_ticks: u32,
        rng: Option<&'a mut dyn RngCore>,
        action_handler: &mut AH,
        condition_handler: &CH,
        observer: &mut O,
    ) -> Status
    where
        AH: ActionHandler<A>,
        CH: ConditionHandler<C>,
        O: Observer,
    {
        self.tick_count = self.tick_count.saturating_add(delta_ticks as u64);
        let mut ctx = Context::new(self.tick_count, delta_ticks, &mut self.blackboard, rng);
        tick_node(
            &self.root,
            0,
            &mut self.states,
            &mut ctx,
            action_handler,
            condition_handler,
            observer,
        )
    }

    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }

    pub fn reset(&mut self) {
        for state in &mut self.states {
            state.reset();
        }
        self.tick_count = 0;
    }

    pub fn reset_all(&mut self) {
        self.reset();
        self.blackboard.clear();
    }

    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    pub fn node_count(&self) -> usize {
        self.states.len()
    }

    pub fn root(&self) -> &BehaviorNode<A, C> {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ActionHandler, BehaviorNode, ConditionHandler, Context, NoOpObserver, Status, TreeBuilder,
    };

    use super::BehaviorTree;

    struct UnitActions;

    impl ActionHandler<u32> for UnitActions {
        fn execute(&mut self, _action: &u32, _ctx: &mut Context) -> Status {
            Status::Success
        }
    }

    struct UnitConditions;

    impl ConditionHandler<u32> for UnitConditions {
        fn check(&self, _condition: &u32, _ctx: &Context) -> bool {
            true
        }
    }

    #[test]
    fn tree_tick_increments_counter() {
        let root: BehaviorNode<u32, u32> = BehaviorNode::Action(1u32);
        let mut tree = BehaviorTree::new(root);
        let mut actions = UnitActions;
        let conditions = UnitConditions;
        let mut observer = NoOpObserver;
        assert_eq!(tree.tick_count(), 0);
        let _ = tree.tick(&mut actions, &conditions, &mut observer);
        assert_eq!(tree.tick_count(), 1);
    }

    #[test]
    fn tree_reset_clears_state() {
        let root: BehaviorNode<u32, u32> = BehaviorNode::Wait(3);
        let mut tree = BehaviorTree::new(root);
        let mut actions = UnitActions;
        let conditions = UnitConditions;
        let mut observer = NoOpObserver;
        assert_eq!(
            tree.tick(&mut actions, &conditions, &mut observer),
            Status::Running
        );
        assert_eq!(tree.tick_count(), 1);
        tree.reset();
        assert_eq!(tree.tick_count(), 0);
        assert_eq!(tree.states[0].tick_counter, 0);
    }

    #[test]
    fn tree_reset_all_clears_blackboard() {
        let root: BehaviorNode<u32, u32> = BehaviorNode::Action(1u32);
        let mut tree = BehaviorTree::new(root);
        tree.blackboard_mut().set_int(1, 99);
        assert!(tree.blackboard().has(1));
        tree.reset_all();
        assert!(!tree.blackboard().has(1));
    }

    #[test]
    fn tree_blackboard_access() {
        let root: BehaviorNode<u32, u32> = TreeBuilder::new().sequence().action(1u32).end().build();
        let mut tree = BehaviorTree::new(root);
        tree.blackboard_mut().set_bool(10, true);
        assert_eq!(tree.blackboard().get_bool(10), Some(true));
    }
}
