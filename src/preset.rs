use alloc::boxed::Box;
use alloc::vec;

use crate::{BehaviorNode, Decorator};

pub fn patrol() -> BehaviorNode<u32, u32> {
    BehaviorNode::Decorator {
        decorator: Decorator::Repeat(u32::MAX),
        child: Box::new(BehaviorNode::Sequence(vec![
            BehaviorNode::Action(4),
            BehaviorNode::Wait(60),
        ])),
    }
}

pub fn combat_melee() -> BehaviorNode<u32, u32> {
    BehaviorNode::Selector(vec![
        BehaviorNode::Sequence(vec![BehaviorNode::Condition(2), BehaviorNode::Action(3)]),
        BehaviorNode::Sequence(vec![BehaviorNode::Condition(1), BehaviorNode::Action(2)]),
        BehaviorNode::Sequence(vec![BehaviorNode::Condition(0), BehaviorNode::Action(1)]),
        BehaviorNode::Action(0),
    ])
}

pub fn guard_post() -> BehaviorNode<u32, u32> {
    BehaviorNode::Selector(vec![
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(0),
            BehaviorNode::Condition(1),
            BehaviorNode::Action(2),
        ]),
        BehaviorNode::Sequence(vec![BehaviorNode::Condition(0), BehaviorNode::Action(1)]),
        BehaviorNode::Sequence(vec![
            BehaviorNode::Decorator {
                decorator: Decorator::Inverter,
                child: Box::new(BehaviorNode::Condition(3)),
            },
            BehaviorNode::Action(4),
        ]),
        BehaviorNode::Action(0),
    ])
}

#[cfg(test)]
mod tests {
    use super::{combat_melee, guard_post, patrol};
    use crate::{BehaviorNode, Decorator};

    #[test]
    fn preset_patrol_loops() {
        let tree = patrol();
        match tree {
            BehaviorNode::Decorator { decorator, child } => {
                assert_eq!(decorator, Decorator::Repeat(u32::MAX));
                assert!(matches!(*child, BehaviorNode::Sequence(_)));
            }
            _ => panic!("unexpected shape"),
        }
    }

    #[test]
    fn preset_patrol_structure() {
        let tree = patrol();
        match tree {
            BehaviorNode::Decorator { child, .. } => match *child {
                BehaviorNode::Sequence(children) => {
                    assert_eq!(children.len(), 2);
                    assert!(matches!(children[0], BehaviorNode::Action(4)));
                    assert!(matches!(children[1], BehaviorNode::Wait(60)));
                }
                _ => panic!("expected sequence"),
            },
            _ => panic!("expected decorator"),
        }
    }

    #[test]
    fn preset_combat_priority() {
        let tree = combat_melee();
        match tree {
            BehaviorNode::Selector(children) => {
                assert_eq!(children.len(), 4);
                match &children[0] {
                    BehaviorNode::Sequence(branch) => {
                        assert!(matches!(branch[0], BehaviorNode::Condition(2)));
                        assert!(matches!(branch[1], BehaviorNode::Action(3)));
                    }
                    _ => panic!("expected sequence"),
                }
            }
            _ => panic!("expected selector"),
        }
    }

    #[test]
    fn preset_combat_flees_when_low() {
        let tree = combat_melee();
        match tree {
            BehaviorNode::Selector(children) => match &children[0] {
                BehaviorNode::Sequence(branch) => {
                    assert!(matches!(branch[0], BehaviorNode::Condition(2)));
                    assert!(matches!(branch[1], BehaviorNode::Action(3)));
                }
                _ => panic!("expected flee branch"),
            },
            _ => panic!("expected selector"),
        }
    }

    #[test]
    fn preset_guard_returns() {
        let tree = guard_post();
        match tree {
            BehaviorNode::Selector(children) => {
                assert_eq!(children.len(), 4);
                match &children[2] {
                    BehaviorNode::Sequence(branch) => {
                        assert!(matches!(branch[1], BehaviorNode::Action(4)));
                    }
                    _ => panic!("expected sequence"),
                }
            }
            _ => panic!("expected selector"),
        }
    }

    #[test]
    fn preset_guard_returns_to_post() {
        let tree = guard_post();
        match tree {
            BehaviorNode::Selector(children) => match &children[2] {
                BehaviorNode::Sequence(branch) => match &branch[0] {
                    BehaviorNode::Decorator { decorator, .. } => {
                        assert_eq!(*decorator, Decorator::Inverter);
                    }
                    _ => panic!("expected inverter"),
                },
                _ => panic!("expected return branch"),
            },
            _ => panic!("expected selector"),
        }
    }
}
