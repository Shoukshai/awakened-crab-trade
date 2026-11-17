//! Stable ordering of nonexclusive native widgets, independent of the desktop.
//! A sequence is equivalent to upstream's increasing numeric wmZorder values.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum Key {
    Stopwatch(u64),
    Image(u64),
}
pub fn reconcile(order: &[Key], known: &[Key]) -> Vec<Key> {
    let mut result = Vec::with_capacity(known.len());
    for key in order.iter().chain(known) {
        if known.contains(key) && !result.contains(key) {
            result.push(*key);
        }
    }
    result
}
pub fn raise(order: &mut Vec<Key>, key: Key) -> bool {
    if order.last() == Some(&key) || !order.contains(&key) {
        return false;
    }
    order.retain(|k| *k != key);
    order.push(key);
    true
}
pub fn saved(order: &[Key], known: &[Key]) -> Vec<Key> {
    let order = reconcile(order, known);
    if order == known {
        Vec::new()
    } else {
        order
    }
}
#[cfg(test)]
mod tests {
    use super::{Key::*, *};
    #[test]
    fn removed_instances_do_not_return_and_new_instances_keep_creation_order() {
        let known = [Stopwatch(1), Image(1), Stopwatch(2)];
        let mut order = reconcile(&[Image(1), Stopwatch(9), Image(1)], &known);
        assert_eq!(order, [Image(1), Stopwatch(1), Stopwatch(2)]);
        assert!(raise(&mut order, Stopwatch(1)));
        assert!(!raise(&mut order, Stopwatch(1)));
        assert!(!raise(&mut order, Image(9)));
        assert_eq!(order, [Image(1), Stopwatch(2), Stopwatch(1)]);
        assert!(saved(&known, &known).is_empty());
        assert_eq!(saved(&order, &known), order);
    }
}
