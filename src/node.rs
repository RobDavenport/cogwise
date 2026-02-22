use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::decorator::Decorator;
use crate::parallel::ParallelPolicy;

/// A node in the behavior tree.
#[derive(Clone, Debug, PartialEq)]
pub enum BehaviorNode<A, C> {
    Sequence(Vec<BehaviorNode<A, C>>),
    Selector(Vec<BehaviorNode<A, C>>),
    Parallel {
        policy: ParallelPolicy,
        children: Vec<BehaviorNode<A, C>>,
    },
    Decorator {
        decorator: Decorator,
        child: Box<BehaviorNode<A, C>>,
    },
    Action(A),
    Condition(C),
    Wait(u32),
    UtilitySelector {
        children: Vec<BehaviorNode<A, C>>,
        utility_ids: Vec<u32>,
    },
    RandomSelector(Vec<BehaviorNode<A, C>>),
    WeightedSelector {
        children: Vec<BehaviorNode<A, C>>,
        weights: Vec<u32>,
    },
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::vec;

    use super::BehaviorNode;
    use crate::decorator::Decorator;
    use crate::parallel::ParallelPolicy;

    #[derive(Clone, Debug, PartialEq)]
    enum A {
        Attack,
        Patrol,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum C {
        Visible,
    }

    #[test]
    fn behavior_node_clone() {
        let tree = BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(C::Visible),
            BehaviorNode::Decorator {
                decorator: Decorator::Repeat(2),
                child: Box::new(BehaviorNode::Action(A::Patrol)),
            },
            BehaviorNode::Parallel {
                policy: ParallelPolicy::RequireOne,
                children: vec![BehaviorNode::Action(A::Attack), BehaviorNode::Wait(2)],
            },
        ]);
        let cloned = tree.clone();
        assert_eq!(tree, cloned);
    }
}
