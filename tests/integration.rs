use alloc::collections::BTreeMap;

use cogwise::{
    preset, ActionHandler, BehaviorNode, BehaviorTree, ConditionHandler, Context, NoOpObserver,
    Status,
};

extern crate alloc;

#[derive(Default)]
struct RecordingActionHandler {
    calls: Vec<u32>,
}

impl ActionHandler<u32> for RecordingActionHandler {
    fn execute(&mut self, action: &u32, _ctx: &mut Context) -> Status {
        self.calls.push(*action);
        Status::Success
    }
}

#[derive(Default)]
struct MapConditionHandler {
    map: BTreeMap<u32, bool>,
}

impl ConditionHandler<u32> for MapConditionHandler {
    fn check(&self, condition: &u32, _ctx: &Context) -> bool {
        self.map.get(condition).copied().unwrap_or(false)
    }
}

#[test]
fn integration_patrol_10_ticks() {
    let root = preset::patrol();
    let mut tree = BehaviorTree::new(root);
    let mut actions = RecordingActionHandler::default();
    let conditions = MapConditionHandler::default();
    let mut observer = NoOpObserver;

    for _ in 0..10 {
        let status = tree.tick(&mut actions, &conditions, &mut observer);
        assert_eq!(status, Status::Running);
    }

    assert_eq!(tree.tick_count(), 10);
    assert!(!actions.calls.is_empty());
}

#[test]
fn integration_combat_scenario() {
    let root = preset::combat_melee();
    let mut tree = BehaviorTree::new(root);
    let mut actions = RecordingActionHandler::default();
    let mut conditions = MapConditionHandler::default();
    let mut observer = NoOpObserver;

    conditions.map.insert(2, false); // low health
    conditions.map.insert(1, true); // in range
    conditions.map.insert(0, true); // visible

    let status = tree.tick(&mut actions, &conditions, &mut observer);
    assert_eq!(status, Status::Success);
    assert_eq!(actions.calls.last().copied(), Some(2));
}

#[test]
fn integration_utility_selector_picks_best() {
    let root = BehaviorNode::UtilitySelector {
        children: vec![BehaviorNode::Action(10), BehaviorNode::Action(20)],
        utility_ids: vec![1, 2],
    };
    let mut tree = BehaviorTree::new(root);
    tree.blackboard_mut().set_float(1, 0.2);
    tree.blackboard_mut().set_float(2, 0.9);

    let mut actions = RecordingActionHandler::default();
    let conditions = MapConditionHandler::default();
    let mut observer = NoOpObserver;
    let status = tree.tick(&mut actions, &conditions, &mut observer);
    assert_eq!(status, Status::Success);
    assert_eq!(actions.calls, vec![20]);
}
