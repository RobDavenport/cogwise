use crate::{
    ActionHandler, BehaviorNode, ConditionHandler, Context, Decorator, Observer, ParallelPolicy,
    Status,
};

#[derive(Clone, Debug, Default)]
pub struct NodeState {
    pub running_child: usize,
    pub tick_counter: u32,
    pub iteration_count: u32,
    pub selected_child: Option<usize>,
    pub random_selection: Option<usize>,
}

impl NodeState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Returns the number of nodes in pre-order traversal.
pub fn assign_ids<A, C>(node: &BehaviorNode<A, C>) -> usize {
    subtree_size(node)
}

pub(crate) fn subtree_size<A, C>(node: &BehaviorNode<A, C>) -> usize {
    match node {
        BehaviorNode::Sequence(children)
        | BehaviorNode::Selector(children)
        | BehaviorNode::RandomSelector(children) => {
            1 + children.iter().map(subtree_size).sum::<usize>()
        }
        BehaviorNode::Parallel { children, .. }
        | BehaviorNode::UtilitySelector { children, .. }
        | BehaviorNode::WeightedSelector { children, .. } => {
            1 + children.iter().map(subtree_size).sum::<usize>()
        }
        BehaviorNode::Decorator { child, .. } => 1 + subtree_size(child),
        BehaviorNode::Action(_) | BehaviorNode::Condition(_) | BehaviorNode::Wait(_) => 1,
    }
}

fn child_id_for_index<A, C>(
    children: &[BehaviorNode<A, C>],
    parent_id: usize,
    index: usize,
) -> usize {
    let mut child_id = parent_id + 1;
    for child in children.iter().take(index) {
        child_id += subtree_size(child);
    }
    child_id
}

fn reset_subtree<A, C>(node: &BehaviorNode<A, C>, node_id: usize, states: &mut [NodeState]) {
    states[node_id].reset();
    match node {
        BehaviorNode::Sequence(children)
        | BehaviorNode::Selector(children)
        | BehaviorNode::RandomSelector(children) => {
            let mut child_id = node_id + 1;
            for child in children {
                reset_subtree(child, child_id, states);
                child_id += subtree_size(child);
            }
        }
        BehaviorNode::Parallel { children, .. }
        | BehaviorNode::UtilitySelector { children, .. }
        | BehaviorNode::WeightedSelector { children, .. } => {
            let mut child_id = node_id + 1;
            for child in children {
                reset_subtree(child, child_id, states);
                child_id += subtree_size(child);
            }
        }
        BehaviorNode::Decorator { child, .. } => {
            reset_subtree(child, node_id + 1, states);
        }
        BehaviorNode::Action(_) | BehaviorNode::Condition(_) | BehaviorNode::Wait(_) => {}
    }
}

pub fn tick_node<A, C, AH, CH, O>(
    node: &BehaviorNode<A, C>,
    node_id: usize,
    states: &mut [NodeState],
    ctx: &mut Context,
    action_handler: &mut AH,
    condition_handler: &CH,
    observer: &mut O,
) -> Status
where
    AH: ActionHandler<A>,
    CH: ConditionHandler<C>,
    O: Observer,
{
    observer.on_enter(node_id);

    let status = match node {
        BehaviorNode::Sequence(children) => {
            let start = states[node_id].running_child.min(children.len());
            let mut child_id = child_id_for_index(children, node_id, start);
            let mut result = Status::Success;

            for (i, child) in children.iter().enumerate().skip(start) {
                let child_status = tick_node(
                    child,
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                );

                match child_status {
                    Status::Running => {
                        states[node_id].running_child = i;
                        result = Status::Running;
                        break;
                    }
                    Status::Failure => {
                        states[node_id].reset();
                        result = Status::Failure;
                        break;
                    }
                    Status::Success => {}
                }
                child_id += subtree_size(child);
            }

            if result == Status::Success {
                states[node_id].reset();
            }

            result
        }
        BehaviorNode::Selector(children) => {
            let start = states[node_id].running_child.min(children.len());
            let mut child_id = child_id_for_index(children, node_id, start);
            let mut result = Status::Failure;

            for (i, child) in children.iter().enumerate().skip(start) {
                let child_status = tick_node(
                    child,
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                );

                match child_status {
                    Status::Running => {
                        states[node_id].running_child = i;
                        result = Status::Running;
                        break;
                    }
                    Status::Success => {
                        states[node_id].reset();
                        result = Status::Success;
                        break;
                    }
                    Status::Failure => {}
                }
                child_id += subtree_size(child);
            }

            if result == Status::Failure {
                states[node_id].reset();
            }

            result
        }
        BehaviorNode::Parallel { policy, children } => {
            let mut success_count = 0usize;
            let mut failure_count = 0usize;
            let mut child_id = node_id + 1;

            for child in children {
                match tick_node(
                    child,
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                ) {
                    Status::Success => success_count += 1,
                    Status::Failure => failure_count += 1,
                    Status::Running => {}
                }
                child_id += subtree_size(child);
            }

            match policy {
                ParallelPolicy::RequireAll => {
                    if failure_count > 0 {
                        Status::Failure
                    } else if success_count == children.len() {
                        Status::Success
                    } else {
                        Status::Running
                    }
                }
                ParallelPolicy::RequireOne => {
                    if success_count > 0 {
                        Status::Success
                    } else if failure_count == children.len() {
                        Status::Failure
                    } else {
                        Status::Running
                    }
                }
                ParallelPolicy::RequireN(n) => {
                    if success_count >= *n {
                        Status::Success
                    } else if children.len().saturating_sub(failure_count) < *n {
                        Status::Failure
                    } else {
                        Status::Running
                    }
                }
            }
        }
        BehaviorNode::Decorator { decorator, child } => {
            let child_id = node_id + 1;
            match decorator {
                Decorator::Inverter => tick_node(
                    child,
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                )
                .invert(),
                Decorator::Repeat(n) => {
                    if *n == 0 {
                        states[node_id].reset();
                        reset_subtree(child, child_id, states);
                        Status::Success
                    } else {
                        let child_status = tick_node(
                            child,
                            child_id,
                            states,
                            ctx,
                            action_handler,
                            condition_handler,
                            observer,
                        );
                        match child_status {
                            Status::Failure => {
                                states[node_id].reset();
                                reset_subtree(child, child_id, states);
                                Status::Failure
                            }
                            Status::Success => {
                                let next = states[node_id].iteration_count.saturating_add(1);
                                states[node_id].iteration_count = next;
                                if next >= *n {
                                    states[node_id].reset();
                                    reset_subtree(child, child_id, states);
                                    Status::Success
                                } else {
                                    reset_subtree(child, child_id, states);
                                    Status::Running
                                }
                            }
                            Status::Running => Status::Running,
                        }
                    }
                }
                Decorator::Retry(n) => {
                    if *n == 0 {
                        states[node_id].reset();
                        reset_subtree(child, child_id, states);
                        Status::Failure
                    } else {
                        let child_status = tick_node(
                            child,
                            child_id,
                            states,
                            ctx,
                            action_handler,
                            condition_handler,
                            observer,
                        );
                        match child_status {
                            Status::Success => {
                                states[node_id].reset();
                                reset_subtree(child, child_id, states);
                                Status::Success
                            }
                            Status::Failure => {
                                let attempts = states[node_id].iteration_count.saturating_add(1);
                                states[node_id].iteration_count = attempts;
                                if attempts >= *n {
                                    states[node_id].reset();
                                    reset_subtree(child, child_id, states);
                                    Status::Failure
                                } else {
                                    reset_subtree(child, child_id, states);
                                    Status::Running
                                }
                            }
                            Status::Running => Status::Running,
                        }
                    }
                }
                Decorator::Cooldown(cooldown_ticks) => {
                    let remaining = states[node_id].tick_counter;
                    if remaining > 0 {
                        let consumed = ctx.delta_ticks().min(remaining);
                        states[node_id].tick_counter = remaining - consumed;
                        Status::Failure
                    } else {
                        let child_status = tick_node(
                            child,
                            child_id,
                            states,
                            ctx,
                            action_handler,
                            condition_handler,
                            observer,
                        );
                        if child_status.is_done() {
                            states[node_id].tick_counter = *cooldown_ticks;
                        }
                        child_status
                    }
                }
                Decorator::Guard(key) => {
                    let allowed = ctx
                        .blackboard()
                        .get(*key)
                        .map(|v| v.is_truthy())
                        .unwrap_or(false);
                    if allowed {
                        tick_node(
                            child,
                            child_id,
                            states,
                            ctx,
                            action_handler,
                            condition_handler,
                            observer,
                        )
                    } else {
                        reset_subtree(child, child_id, states);
                        Status::Failure
                    }
                }
                Decorator::UntilSuccess => {
                    let child_status = tick_node(
                        child,
                        child_id,
                        states,
                        ctx,
                        action_handler,
                        condition_handler,
                        observer,
                    );
                    match child_status {
                        Status::Success => {
                            states[node_id].reset();
                            reset_subtree(child, child_id, states);
                            Status::Success
                        }
                        Status::Failure => {
                            reset_subtree(child, child_id, states);
                            Status::Running
                        }
                        Status::Running => Status::Running,
                    }
                }
                Decorator::UntilFail => {
                    let child_status = tick_node(
                        child,
                        child_id,
                        states,
                        ctx,
                        action_handler,
                        condition_handler,
                        observer,
                    );
                    match child_status {
                        Status::Failure => {
                            states[node_id].reset();
                            reset_subtree(child, child_id, states);
                            Status::Failure
                        }
                        Status::Success => {
                            reset_subtree(child, child_id, states);
                            Status::Running
                        }
                        Status::Running => Status::Running,
                    }
                }
                Decorator::Timeout(max_ticks) => {
                    let elapsed = states[node_id].tick_counter.saturating_add(ctx.delta_ticks());
                    states[node_id].tick_counter = elapsed;
                    if elapsed >= *max_ticks {
                        states[node_id].reset();
                        reset_subtree(child, child_id, states);
                        Status::Failure
                    } else {
                        let child_status = tick_node(
                            child,
                            child_id,
                            states,
                            ctx,
                            action_handler,
                            condition_handler,
                            observer,
                        );
                        if child_status.is_done() {
                            states[node_id].reset();
                        }
                        child_status
                    }
                }
                Decorator::ForceSuccess => {
                    let child_status = tick_node(
                        child,
                        child_id,
                        states,
                        ctx,
                        action_handler,
                        condition_handler,
                        observer,
                    );
                    if child_status == Status::Running {
                        Status::Running
                    } else {
                        Status::Success
                    }
                }
                Decorator::ForceFailure => {
                    let child_status = tick_node(
                        child,
                        child_id,
                        states,
                        ctx,
                        action_handler,
                        condition_handler,
                        observer,
                    );
                    if child_status == Status::Running {
                        Status::Running
                    } else {
                        Status::Failure
                    }
                }
            }
        }
        BehaviorNode::Action(action_id) => action_handler.execute(action_id, ctx),
        BehaviorNode::Condition(condition_id) => {
            if condition_handler.check(condition_id, ctx) {
                Status::Success
            } else {
                Status::Failure
            }
        }
        BehaviorNode::Wait(ticks) => {
            if *ticks == 0 {
                states[node_id].reset();
                Status::Success
            } else {
                let elapsed = states[node_id].tick_counter.saturating_add(ctx.delta_ticks());
                states[node_id].tick_counter = elapsed;
                if elapsed >= *ticks {
                    states[node_id].reset();
                    Status::Success
                } else {
                    Status::Running
                }
            }
        }
        BehaviorNode::UtilitySelector {
            children,
            utility_ids,
        } => {
            if children.is_empty() || children.len() != utility_ids.len() {
                states[node_id].reset();
                Status::Failure
            } else if let Some(selected) = states[node_id].selected_child {
                if selected >= children.len() {
                    states[node_id].reset();
                    Status::Failure
                } else {
                    let child_id = child_id_for_index(children, node_id, selected);
                    let child_status = tick_node(
                        &children[selected],
                        child_id,
                        states,
                        ctx,
                        action_handler,
                        condition_handler,
                        observer,
                    );
                    if child_status != Status::Running {
                        states[node_id].reset();
                    }
                    child_status
                }
            } else {
                let mut best_idx = 0usize;
                let mut best_score = f32::MIN;
                for (i, utility_key) in utility_ids.iter().enumerate() {
                    let score = ctx
                        .blackboard()
                        .get(*utility_key)
                        .map(|v| v.to_score_f32())
                        .unwrap_or(0.0);
                    observer.on_utility_score(i, score);
                    if score > best_score {
                        best_score = score;
                        best_idx = i;
                    }
                }

                states[node_id].selected_child = Some(best_idx);
                let child_id = child_id_for_index(children, node_id, best_idx);
                let child_status = tick_node(
                    &children[best_idx],
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                );
                if child_status != Status::Running {
                    states[node_id].reset();
                }
                child_status
            }
        }
        BehaviorNode::RandomSelector(children) => {
            if children.is_empty() {
                states[node_id].reset();
                Status::Failure
            } else {
                let selected = match states[node_id].random_selection {
                    Some(idx) if idx < children.len() => idx,
                    _ => {
                        let idx = (ctx.rng().next_u32() as usize) % children.len();
                        states[node_id].random_selection = Some(idx);
                        idx
                    }
                };

                let child_id = child_id_for_index(children, node_id, selected);
                let child_status = tick_node(
                    &children[selected],
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                );
                if child_status != Status::Running {
                    states[node_id].reset();
                }
                child_status
            }
        }
        BehaviorNode::WeightedSelector { children, weights } => {
            if children.is_empty() || children.len() != weights.len() {
                states[node_id].reset();
                Status::Failure
            } else {
                let selected = match states[node_id].random_selection {
                    Some(idx) if idx < children.len() => idx,
                    _ => {
                        let total_weight: u32 = weights.iter().copied().sum();
                        if total_weight == 0 {
                            states[node_id].reset();
                            observer.on_exit(node_id, Status::Failure);
                            return Status::Failure;
                        }
                        let mut roll = ctx.rng().next_u32() % total_weight;
                        let mut idx = 0usize;
                        for (i, weight) in weights.iter().enumerate() {
                            if roll < *weight {
                                idx = i;
                                break;
                            }
                            roll = roll.saturating_sub(*weight);
                        }
                        states[node_id].random_selection = Some(idx);
                        idx
                    }
                };

                let child_id = child_id_for_index(children, node_id, selected);
                let child_status = tick_node(
                    &children[selected],
                    child_id,
                    states,
                    ctx,
                    action_handler,
                    condition_handler,
                    observer,
                );
                if child_status != Status::Running {
                    states[node_id].reset();
                }
                child_status
            }
        }
    };

    observer.on_exit(node_id, status);
    status
}

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use alloc::collections::BTreeMap;
    use alloc::vec;
    use alloc::vec::Vec;
    use rand_core::{Error, RngCore};

    use super::{assign_ids, tick_node, NodeState};
    use crate::{
        ActionHandler, BehaviorNode, Blackboard, ConditionHandler, Context, Decorator,
        NoOpObserver, ParallelPolicy, Status,
    };

    #[derive(Default)]
    struct ScriptedActionHandler {
        scripted: BTreeMap<u32, Vec<Status>>,
        calls: Vec<u32>,
    }

    impl ScriptedActionHandler {
        fn with_script(scripted: BTreeMap<u32, Vec<Status>>) -> Self {
            Self {
                scripted,
                calls: Vec::new(),
            }
        }
    }

    impl ActionHandler<u32> for ScriptedActionHandler {
        fn execute(&mut self, action: &u32, _ctx: &mut Context) -> Status {
            self.calls.push(*action);
            if let Some(queue) = self.scripted.get_mut(action) {
                if queue.is_empty() {
                    Status::Success
                } else {
                    queue.remove(0)
                }
            } else {
                Status::Success
            }
        }
    }

    #[derive(Default)]
    struct ScriptedConditionHandler {
        values: BTreeMap<u32, bool>,
    }

    impl ConditionHandler<u32> for ScriptedConditionHandler {
        fn check(&self, condition: &u32, _ctx: &Context) -> bool {
            self.values.get(condition).copied().unwrap_or(false)
        }
    }

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

    fn states_for(node: &BehaviorNode<u32, u32>) -> Vec<NodeState> {
        vec![NodeState::default(); assign_ids(node)]
    }

    fn tick_once<'a>(
        node: &BehaviorNode<u32, u32>,
        states: &mut [NodeState],
        bb: &'a mut Blackboard,
        rng: Option<&'a mut dyn RngCore>,
        action_handler: &mut ScriptedActionHandler,
        condition_handler: &ScriptedConditionHandler,
    ) -> Status {
        let mut ctx = Context::new(1, 1, bb, rng);
        let mut observer = NoOpObserver;
        tick_node(
            node,
            0,
            states,
            &mut ctx,
            action_handler,
            condition_handler,
            &mut observer,
        )
    }

    #[test]
    fn tick_sequence_all_success() {
        let node = BehaviorNode::Sequence(vec![
            BehaviorNode::Action(1),
            BehaviorNode::Action(2),
            BehaviorNode::Action(3),
        ]);
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Success);
        assert_eq!(actions.calls, vec![1, 2, 3]);
    }

    #[test]
    fn tick_sequence_first_failure() {
        let node = BehaviorNode::Sequence(vec![
            BehaviorNode::Action(1),
            BehaviorNode::Action(2),
            BehaviorNode::Action(3),
        ]);
        let mut script = BTreeMap::new();
        script.insert(2, vec![Status::Failure]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Failure);
        assert_eq!(actions.calls, vec![1, 2]);
    }

    #[test]
    fn tick_sequence_resumes_running() {
        let node = BehaviorNode::Sequence(vec![
            BehaviorNode::Action(1),
            BehaviorNode::Action(2),
            BehaviorNode::Action(3),
        ]);
        let mut script = BTreeMap::new();
        script.insert(2, vec![Status::Running, Status::Success]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();

        let first = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        let second = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );

        assert_eq!(first, Status::Running);
        assert_eq!(second, Status::Success);
        assert_eq!(actions.calls, vec![1, 2, 2, 3]);
        assert_eq!(actions.calls.iter().filter(|a| **a == 1).count(), 1);
    }

    #[test]
    fn tick_selector_first_success() {
        let node = BehaviorNode::Selector(vec![BehaviorNode::Action(1), BehaviorNode::Action(2)]);
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Success);
        assert_eq!(actions.calls, vec![1]);
    }

    #[test]
    fn tick_selector_all_failure() {
        let node = BehaviorNode::Selector(vec![BehaviorNode::Action(1), BehaviorNode::Action(2)]);
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Failure]);
        script.insert(2, vec![Status::Failure]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();

        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Failure);
        assert_eq!(actions.calls, vec![1, 2]);
    }

    #[test]
    fn tick_selector_resumes_running() {
        let node = BehaviorNode::Selector(vec![
            BehaviorNode::Action(1),
            BehaviorNode::Action(2),
            BehaviorNode::Action(3),
        ]);
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Failure]);
        script.insert(2, vec![Status::Running, Status::Success]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();

        let first = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        let second = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );

        assert_eq!(first, Status::Running);
        assert_eq!(second, Status::Success);
        assert_eq!(actions.calls.iter().filter(|a| **a == 1).count(), 1);
    }

    #[test]
    fn tick_parallel_require_all_success() {
        let node = BehaviorNode::Parallel {
            policy: ParallelPolicy::RequireAll,
            children: vec![BehaviorNode::Action(1), BehaviorNode::Action(2)],
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn tick_parallel_require_all_one_failure() {
        let node = BehaviorNode::Parallel {
            policy: ParallelPolicy::RequireAll,
            children: vec![BehaviorNode::Action(1), BehaviorNode::Action(2)],
        };
        let mut script = BTreeMap::new();
        script.insert(2, vec![Status::Failure]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn tick_parallel_require_one_success() {
        let node = BehaviorNode::Parallel {
            policy: ParallelPolicy::RequireOne,
            children: vec![BehaviorNode::Action(1), BehaviorNode::Action(2)],
        };
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Failure]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn tick_parallel_require_n() {
        let node_success = BehaviorNode::Parallel {
            policy: ParallelPolicy::RequireN(2),
            children: vec![
                BehaviorNode::Action(1),
                BehaviorNode::Action(2),
                BehaviorNode::Action(3),
            ],
        };
        let mut script_success = BTreeMap::new();
        script_success.insert(3, vec![Status::Failure]);
        let mut actions_success = ScriptedActionHandler::with_script(script_success);
        let mut states_success = states_for(&node_success);
        let mut bb = Blackboard::new();
        let conditions = ScriptedConditionHandler::default();
        let status_success = tick_once(
            &node_success,
            &mut states_success,
            &mut bb,
            None,
            &mut actions_success,
            &conditions,
        );
        assert_eq!(status_success, Status::Success);

        let node_failure = BehaviorNode::Parallel {
            policy: ParallelPolicy::RequireN(3),
            children: vec![
                BehaviorNode::Action(1),
                BehaviorNode::Action(2),
                BehaviorNode::Action(3),
            ],
        };
        let mut script_failure = BTreeMap::new();
        script_failure.insert(1, vec![Status::Failure]);
        script_failure.insert(2, vec![Status::Failure]);
        let mut actions_failure = ScriptedActionHandler::with_script(script_failure);
        let mut states_failure = states_for(&node_failure);
        let status_failure = tick_once(
            &node_failure,
            &mut states_failure,
            &mut bb,
            None,
            &mut actions_failure,
            &conditions,
        );
        assert_eq!(status_failure, Status::Failure);
    }

    #[test]
    fn tick_decorator_inverter() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Inverter,
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Success]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn tick_decorator_repeat() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Repeat(2),
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let first = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        let second = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(first, Status::Running);
        assert_eq!(second, Status::Success);
    }

    #[test]
    fn tick_decorator_retry() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Retry(3),
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Failure, Status::Failure, Status::Success]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
    }

    #[test]
    fn tick_decorator_cooldown() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Cooldown(2),
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();

        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Failure
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Failure
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
    }

    #[test]
    fn tick_decorator_guard_pass() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Guard(10),
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        bb.set_bool(10, true);
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
        assert_eq!(actions.calls, vec![1]);
    }

    #[test]
    fn tick_decorator_guard_fail() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Guard(10),
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        bb.set_bool(10, false);
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Failure
        );
        assert!(actions.calls.is_empty());
    }

    #[test]
    fn tick_decorator_until_success() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::UntilSuccess,
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Failure, Status::Failure, Status::Success]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
    }

    #[test]
    fn tick_decorator_until_fail() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::UntilFail,
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Success, Status::Success, Status::Failure]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();

        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Failure
        );
    }

    #[test]
    fn tick_decorator_timeout() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::Timeout(2),
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Running, Status::Running, Status::Running]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();

        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Failure
        );
    }

    #[test]
    fn tick_decorator_force_success() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::ForceSuccess,
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Failure]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
    }

    #[test]
    fn tick_decorator_force_failure() {
        let node = BehaviorNode::Decorator {
            decorator: Decorator::ForceFailure,
            child: Box::new(BehaviorNode::Action(1)),
        };
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Failure
        );
    }

    #[test]
    fn tick_wait_counts_ticks() {
        let node = BehaviorNode::Wait(3);
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                None,
                &mut actions,
                &conditions
            ),
            Status::Success
        );
    }

    #[test]
    fn tick_action_delegates() {
        let node = BehaviorNode::Action(5);
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Success);
        assert_eq!(actions.calls, vec![5]);
    }

    #[test]
    fn tick_condition_true() {
        let node = BehaviorNode::Condition(10);
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let mut conditions = ScriptedConditionHandler::default();
        conditions.values.insert(10, true);
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Success);
    }

    #[test]
    fn tick_condition_false() {
        let node = BehaviorNode::Condition(10);
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let mut conditions = ScriptedConditionHandler::default();
        conditions.values.insert(10, false);
        let status = tick_once(
            &node,
            &mut states,
            &mut bb,
            None,
            &mut actions,
            &conditions,
        );
        assert_eq!(status, Status::Failure);
    }

    #[test]
    fn tick_random_selector_persists_running() {
        let node =
            BehaviorNode::RandomSelector(vec![BehaviorNode::Action(1), BehaviorNode::Action(2)]);
        let mut script = BTreeMap::new();
        script.insert(1, vec![Status::Running, Status::Success]);
        script.insert(2, vec![Status::Success]);
        let mut actions = ScriptedActionHandler::with_script(script);
        let conditions = ScriptedConditionHandler::default();
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut rng = SeqRng::new(vec![0, 1]);

        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                Some(&mut rng),
                &mut actions,
                &conditions
            ),
            Status::Running
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                Some(&mut rng),
                &mut actions,
                &conditions
            ),
            Status::Success
        );
        assert_eq!(actions.calls, vec![1, 1]);
    }

    #[test]
    fn tick_weighted_selector_respects_weights() {
        let node = BehaviorNode::WeightedSelector {
            children: vec![BehaviorNode::Action(1), BehaviorNode::Action(2)],
            weights: vec![1, 9],
        };
        let mut states = states_for(&node);
        let mut bb = Blackboard::new();
        let mut actions = ScriptedActionHandler::default();
        let conditions = ScriptedConditionHandler::default();
        let mut rng = SeqRng::new(vec![0, 9]);

        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                Some(&mut rng),
                &mut actions,
                &conditions
            ),
            Status::Success
        );
        assert_eq!(
            tick_once(
                &node,
                &mut states,
                &mut bb,
                Some(&mut rng),
                &mut actions,
                &conditions
            ),
            Status::Success
        );
        assert_eq!(actions.calls, vec![1, 2]);
    }
}
