use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::{BehaviorNode, Decorator, ParallelPolicy};

pub struct TreeBuilder<A, C> {
    stack: Vec<BuilderFrame<A, C>>,
    root: Option<BehaviorNode<A, C>>,
    pending_decorators: Vec<Decorator>,
}

struct BuilderFrame<A, C> {
    node_type: CompositeType,
    children: Vec<BehaviorNode<A, C>>,
    metadata: FrameMetadata,
}

enum CompositeType {
    Sequence,
    Selector,
    Parallel(ParallelPolicy),
    RandomSelector,
    WeightedSelector,
}

#[derive(Default)]
struct FrameMetadata {
    weights: Vec<u32>,
}

impl<A, C> TreeBuilder<A, C> {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            root: None,
            pending_decorators: Vec::new(),
        }
    }

    pub fn sequence(mut self) -> Self {
        self.stack.push(BuilderFrame {
            node_type: CompositeType::Sequence,
            children: Vec::new(),
            metadata: FrameMetadata::default(),
        });
        self
    }

    pub fn selector(mut self) -> Self {
        self.stack.push(BuilderFrame {
            node_type: CompositeType::Selector,
            children: Vec::new(),
            metadata: FrameMetadata::default(),
        });
        self
    }

    pub fn parallel(mut self, policy: ParallelPolicy) -> Self {
        self.stack.push(BuilderFrame {
            node_type: CompositeType::Parallel(policy),
            children: Vec::new(),
            metadata: FrameMetadata::default(),
        });
        self
    }

    pub fn random_selector(mut self) -> Self {
        self.stack.push(BuilderFrame {
            node_type: CompositeType::RandomSelector,
            children: Vec::new(),
            metadata: FrameMetadata::default(),
        });
        self
    }

    pub fn weighted_selector(mut self) -> Self {
        self.stack.push(BuilderFrame {
            node_type: CompositeType::WeightedSelector,
            children: Vec::new(),
            metadata: FrameMetadata::default(),
        });
        self
    }

    pub fn action(mut self, action: A) -> Self {
        self.push_node(BehaviorNode::Action(action));
        self
    }

    pub fn condition(mut self, condition: C) -> Self {
        self.push_node(BehaviorNode::Condition(condition));
        self
    }

    pub fn wait(mut self, ticks: u32) -> Self {
        self.push_node(BehaviorNode::Wait(ticks));
        self
    }

    pub fn decorator(mut self, decorator: Decorator) -> Self {
        self.pending_decorators.push(decorator);
        self
    }

    pub fn weight(mut self, weight: u32) -> Self {
        let frame = self
            .stack
            .last_mut()
            .expect("weight() requires an open composite");
        match frame.node_type {
            CompositeType::WeightedSelector => frame.metadata.weights.push(weight),
            _ => panic!("weight() is only valid inside weighted_selector()"),
        }
        self
    }

    pub fn end(mut self) -> Self {
        let frame = self
            .stack
            .pop()
            .expect("end() called with no open composite");
        let mut node = match frame.node_type {
            CompositeType::Sequence => BehaviorNode::Sequence(frame.children),
            CompositeType::Selector => BehaviorNode::Selector(frame.children),
            CompositeType::Parallel(policy) => BehaviorNode::Parallel {
                policy,
                children: frame.children,
            },
            CompositeType::RandomSelector => BehaviorNode::RandomSelector(frame.children),
            CompositeType::WeightedSelector => {
                if frame.children.len() != frame.metadata.weights.len() {
                    panic!(
                        "weighted_selector children/weights mismatch: {} children, {} weights",
                        frame.children.len(),
                        frame.metadata.weights.len()
                    );
                }
                BehaviorNode::WeightedSelector {
                    children: frame.children,
                    weights: frame.metadata.weights,
                }
            }
        };

        node = self.wrap_with_pending_decorators(node);
        if let Some(parent) = self.stack.last_mut() {
            parent.children.push(node);
        } else {
            self.set_root(node);
        }
        self
    }

    pub fn build(mut self) -> BehaviorNode<A, C> {
        if !self.stack.is_empty() {
            panic!("build() with unclosed composites: {}", self.stack.len());
        }
        if !self.pending_decorators.is_empty() {
            panic!(
                "build() with dangling decorators: {}",
                self.pending_decorators.len()
            );
        }
        self.root.take().expect("build() requires at least one node")
    }

    fn push_node(&mut self, node: BehaviorNode<A, C>) {
        let node = self.wrap_with_pending_decorators(node);
        if let Some(frame) = self.stack.last_mut() {
            frame.children.push(node);
        } else {
            self.set_root(node);
        }
    }

    fn wrap_with_pending_decorators(&mut self, mut node: BehaviorNode<A, C>) -> BehaviorNode<A, C> {
        while let Some(decorator) = self.pending_decorators.pop() {
            node = BehaviorNode::Decorator {
                decorator,
                child: Box::new(node),
            };
        }
        node
    }

    fn set_root(&mut self, node: BehaviorNode<A, C>) {
        if self.root.is_some() {
            panic!("multiple root nodes built without enclosing composite");
        }
        self.root = Some(node);
    }
}

impl<A, C> Default for TreeBuilder<A, C> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use crate::{BehaviorNode, Decorator, TreeBuilder};

    #[test]
    fn builder_simple_sequence() {
        let tree: BehaviorNode<u32, u32> = TreeBuilder::new()
            .sequence()
            .action(1u32)
            .action(2u32)
            .end()
            .build();
        match tree {
            BehaviorNode::Sequence(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], BehaviorNode::Action(1)));
                assert!(matches!(children[1], BehaviorNode::Action(2)));
            }
            _ => panic!("expected sequence"),
        }
    }

    #[test]
    fn builder_nested() {
        let tree: BehaviorNode<u32, u32> = TreeBuilder::new()
            .selector()
            .sequence()
            .condition(1u32)
            .action(2u32)
            .end()
            .action(3u32)
            .end()
            .build();
        match tree {
            BehaviorNode::Selector(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], BehaviorNode::Sequence(_)));
                assert!(matches!(children[1], BehaviorNode::Action(3)));
            }
            _ => panic!("expected selector"),
        }
    }

    #[test]
    fn builder_nested_composites() {
        let tree: BehaviorNode<u32, u32> = TreeBuilder::new()
            .sequence()
            .selector()
            .action(1u32)
            .action(2u32)
            .end()
            .wait(1)
            .end()
            .build();
        match tree {
            BehaviorNode::Sequence(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], BehaviorNode::Selector(_)));
                assert!(matches!(children[1], BehaviorNode::Wait(1)));
            }
            _ => panic!("expected sequence"),
        }
    }

    #[test]
    fn builder_with_decorator() {
        let tree: BehaviorNode<u32, u32> = TreeBuilder::new()
            .sequence()
            .decorator(Decorator::Inverter)
            .condition(1u32)
            .end()
            .build();
        match tree {
            BehaviorNode::Sequence(children) => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    BehaviorNode::Decorator { decorator, child } => {
                        assert_eq!(*decorator, Decorator::Inverter);
                        assert!(matches!(**child, BehaviorNode::Condition(1)));
                    }
                    _ => panic!("expected decorator"),
                }
            }
            _ => panic!("expected sequence"),
        }
    }

    #[test]
    fn builder_weighted_selector() {
        let tree: BehaviorNode<u32, u32> = TreeBuilder::new()
            .weighted_selector()
            .action(1u32)
            .weight(10)
            .action(2u32)
            .weight(1)
            .end()
            .build();
        match tree {
            BehaviorNode::WeightedSelector { children, weights } => {
                assert_eq!(children.len(), 2);
                assert_eq!(weights, vec![10, 1]);
            }
            _ => panic!("expected weighted selector"),
        }
    }
}
