use alloc::vec::Vec;

use crate::{BlackboardValue, Status};

pub trait Observer {
    fn on_enter(&mut self, _node_id: usize) {}
    fn on_exit(&mut self, _node_id: usize, _status: Status) {}
    fn on_blackboard_write(&mut self, _key: u32, _value: BlackboardValue) {}
    fn on_utility_score(&mut self, _action_index: usize, _score: f32) {}
}

#[derive(Default)]
pub struct NoOpObserver;

impl Observer for NoOpObserver {}

#[derive(Default)]
pub struct RecordingObserver {
    pub events: Vec<ObserverEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObserverEvent {
    Enter(usize),
    Exit(usize, Status),
    BlackboardWrite(u32, BlackboardValue),
    UtilityScore(usize, f32),
}

impl Observer for RecordingObserver {
    fn on_enter(&mut self, node_id: usize) {
        self.events.push(ObserverEvent::Enter(node_id));
    }

    fn on_exit(&mut self, node_id: usize, status: Status) {
        self.events.push(ObserverEvent::Exit(node_id, status));
    }

    fn on_blackboard_write(&mut self, key: u32, value: BlackboardValue) {
        self.events.push(ObserverEvent::BlackboardWrite(key, value));
    }

    fn on_utility_score(&mut self, action_index: usize, score: f32) {
        self.events
            .push(ObserverEvent::UtilityScore(action_index, score));
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{NoOpObserver, Observer, ObserverEvent, RecordingObserver};
    use crate::{BlackboardValue, Status};

    #[test]
    fn observer_records_events() {
        let mut observer = RecordingObserver::default();
        observer.on_enter(3);
        observer.on_exit(3, Status::Success);
        observer.on_blackboard_write(5, BlackboardValue::Int(7));
        observer.on_utility_score(1, 0.75);

        assert_eq!(
            observer.events,
            vec![
                ObserverEvent::Enter(3),
                ObserverEvent::Exit(3, Status::Success),
                ObserverEvent::BlackboardWrite(5, BlackboardValue::Int(7)),
                ObserverEvent::UtilityScore(1, 0.75),
            ]
        );
    }

    #[test]
    fn observer_noop_compiles() {
        let mut observer = NoOpObserver;
        observer.on_enter(0);
        observer.on_exit(0, Status::Running);
    }
}
