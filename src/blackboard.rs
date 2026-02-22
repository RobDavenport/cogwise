use alloc::collections::BTreeMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlackboardValue {
    Int(i32),
    Fixed(i32),
    Bool(bool),
    Entity(u32),
    Vec2(i32, i32),
}

impl BlackboardValue {
    pub fn from_f32(v: f32) -> Self {
        BlackboardValue::Fixed((v * 1000.0) as i32)
    }

    pub fn as_int(self) -> Option<i32> {
        match self {
            BlackboardValue::Int(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_float(self) -> Option<f32> {
        match self {
            BlackboardValue::Fixed(v) => Some((v as f32) / 1000.0),
            _ => None,
        }
    }

    pub fn as_bool(self) -> Option<bool> {
        match self {
            BlackboardValue::Bool(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_entity(self) -> Option<u32> {
        match self {
            BlackboardValue::Entity(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_vec2(self) -> Option<(i32, i32)> {
        match self {
            BlackboardValue::Vec2(x, y) => Some((x, y)),
            _ => None,
        }
    }

    pub fn is_truthy(self) -> bool {
        match self {
            BlackboardValue::Int(v) => v != 0,
            BlackboardValue::Fixed(v) => v != 0,
            BlackboardValue::Bool(v) => v,
            BlackboardValue::Entity(v) => v != 0,
            BlackboardValue::Vec2(x, y) => x != 0 || y != 0,
        }
    }

    pub(crate) fn to_score_f32(self) -> f32 {
        match self {
            BlackboardValue::Int(v) => v as f32,
            BlackboardValue::Fixed(v) => (v as f32) / 1000.0,
            BlackboardValue::Bool(v) => {
                if v {
                    1.0
                } else {
                    0.0
                }
            }
            BlackboardValue::Entity(v) => v as f32,
            BlackboardValue::Vec2(x, y) => {
                let xf = x as f32;
                let yf = y as f32;
                libm::sqrtf(xf * xf + yf * yf)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Blackboard {
    entries: BTreeMap<u32, BlackboardValue>,
}

impl Blackboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: u32) -> Option<BlackboardValue> {
        self.entries.get(&key).copied()
    }

    pub fn get_int(&self, key: u32) -> Option<i32> {
        self.get(key).and_then(BlackboardValue::as_int)
    }

    pub fn get_float(&self, key: u32) -> Option<f32> {
        self.get(key).and_then(BlackboardValue::as_float)
    }

    pub fn get_bool(&self, key: u32) -> Option<bool> {
        self.get(key).and_then(BlackboardValue::as_bool)
    }

    pub fn get_entity(&self, key: u32) -> Option<u32> {
        self.get(key).and_then(BlackboardValue::as_entity)
    }

    pub fn get_vec2(&self, key: u32) -> Option<(i32, i32)> {
        self.get(key).and_then(BlackboardValue::as_vec2)
    }

    pub fn set(&mut self, key: u32, value: BlackboardValue) {
        self.entries.insert(key, value);
    }

    pub fn set_int(&mut self, key: u32, value: i32) {
        self.set(key, BlackboardValue::Int(value));
    }

    pub fn set_float(&mut self, key: u32, value: f32) {
        self.set(key, BlackboardValue::from_f32(value));
    }

    pub fn set_bool(&mut self, key: u32, value: bool) {
        self.set(key, BlackboardValue::Bool(value));
    }

    pub fn set_entity(&mut self, key: u32, value: u32) {
        self.set(key, BlackboardValue::Entity(value));
    }

    pub fn set_vec2(&mut self, key: u32, x: i32, y: i32) {
        self.set(key, BlackboardValue::Vec2(x, y));
    }

    pub fn has(&self, key: u32) -> bool {
        self.entries.contains_key(&key)
    }

    pub fn remove(&mut self, key: u32) -> Option<BlackboardValue> {
        self.entries.remove(&key)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Blackboard, BlackboardValue};

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() < 1.0e-6, "{left} != {right}");
    }

    #[test]
    fn blackboard_set_get_int() {
        let mut bb = Blackboard::new();
        bb.set_int(1, 42);
        assert_eq!(bb.get_int(1), Some(42));
    }

    #[test]
    fn blackboard_set_get_fixed() {
        let mut bb = Blackboard::new();
        bb.set_float(1, 1.25);
        approx_eq(bb.get_float(1).unwrap_or_default(), 1.25);
    }

    #[test]
    fn blackboard_set_get_bool() {
        let mut bb = Blackboard::new();
        bb.set_bool(1, true);
        assert_eq!(bb.get_bool(1), Some(true));
    }

    #[test]
    fn blackboard_set_get_entity() {
        let mut bb = Blackboard::new();
        bb.set_entity(1, 99);
        assert_eq!(bb.get_entity(1), Some(99));
    }

    #[test]
    fn blackboard_set_get_vec2() {
        let mut bb = Blackboard::new();
        bb.set_vec2(1, 4, -2);
        assert_eq!(bb.get_vec2(1), Some((4, -2)));
    }

    #[test]
    fn blackboard_set_get_all_types() {
        let mut bb = Blackboard::new();
        bb.set(1, BlackboardValue::Int(3));
        bb.set(2, BlackboardValue::Fixed(3500));
        bb.set(3, BlackboardValue::Bool(true));
        bb.set(4, BlackboardValue::Entity(7));
        bb.set(5, BlackboardValue::Vec2(9, 1));
        assert_eq!(bb.get(1), Some(BlackboardValue::Int(3)));
        assert_eq!(bb.get(2), Some(BlackboardValue::Fixed(3500)));
        assert_eq!(bb.get(3), Some(BlackboardValue::Bool(true)));
        assert_eq!(bb.get(4), Some(BlackboardValue::Entity(7)));
        assert_eq!(bb.get(5), Some(BlackboardValue::Vec2(9, 1)));
    }

    #[test]
    fn blackboard_overwrite() {
        let mut bb = Blackboard::new();
        bb.set_int(1, 10);
        bb.set_int(1, 20);
        assert_eq!(bb.get_int(1), Some(20));
    }

    #[test]
    fn blackboard_remove() {
        let mut bb = Blackboard::new();
        bb.set_int(1, 7);
        assert_eq!(bb.remove(1), Some(BlackboardValue::Int(7)));
        assert_eq!(bb.get(1), None);
    }

    #[test]
    fn blackboard_clear() {
        let mut bb = Blackboard::new();
        bb.set_int(1, 1);
        bb.set_int(2, 2);
        bb.clear();
        assert!(bb.is_empty());
    }

    #[test]
    fn blackboard_has() {
        let mut bb = Blackboard::new();
        bb.set_bool(9, true);
        assert!(bb.has(9));
        assert!(!bb.has(8));
    }

    #[test]
    fn blackboard_is_truthy_int() {
        assert!(!BlackboardValue::Int(0).is_truthy());
        assert!(BlackboardValue::Int(1).is_truthy());
    }

    #[test]
    fn blackboard_is_truthy_bool() {
        assert!(!BlackboardValue::Bool(false).is_truthy());
        assert!(BlackboardValue::Bool(true).is_truthy());
    }

    #[test]
    fn blackboard_is_truthy() {
        assert!(!BlackboardValue::Int(0).is_truthy());
        assert!(BlackboardValue::Int(-3).is_truthy());
        assert!(!BlackboardValue::Fixed(0).is_truthy());
        assert!(BlackboardValue::Fixed(1).is_truthy());
        assert!(!BlackboardValue::Bool(false).is_truthy());
        assert!(BlackboardValue::Bool(true).is_truthy());
        assert!(!BlackboardValue::Entity(0).is_truthy());
        assert!(BlackboardValue::Entity(44).is_truthy());
        assert!(!BlackboardValue::Vec2(0, 0).is_truthy());
        assert!(BlackboardValue::Vec2(0, 1).is_truthy());
    }

    #[test]
    fn blackboard_from_f32() {
        assert_eq!(BlackboardValue::from_f32(1.5), BlackboardValue::Fixed(1500));
    }
}
