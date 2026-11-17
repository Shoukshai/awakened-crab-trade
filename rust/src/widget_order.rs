//! Stable ordering of nonexclusive native widgets, independent of the desktop.
//! A sequence is equivalent to upstream's increasing numeric wmZorder values.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "id", rename_all = "snake_case")]
pub enum Key {
    Menu,
    Stopwatch(u64),
    Image(u64),
    Stash(u64),
}
pub fn reconcile(order: &[Key], known: &[Key]) -> Vec<Key> {
    let mut result = Vec::with_capacity(known.len());
    if known.contains(&Key::Menu) && !order.contains(&Key::Menu) {
        result.push(Key::Menu);
    }
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
    let order_first_menu = order.first() == Some(&Key::Menu);
    if order == known {
        Vec::new()
    } else {
        order
            .into_iter()
            .filter(|k| !(*k == Key::Menu && order_first_menu))
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::{Key::*, *};
    #[test]
    fn old_orders_keep_the_menu_at_its_default_depth_and_explicit_menu_raises_roundtrip() {
        let known = [Menu, Stopwatch(1), Image(1)];
        let old = [Image(1), Stopwatch(1)];
        let mut order = reconcile(&old, &known);
        assert_eq!(order, [Menu, Image(1), Stopwatch(1)]);
        assert_eq!(saved(&order, &known), old);
        raise(&mut order, Menu);
        assert_eq!(reconcile(&saved(&order, &known), &known), order);
    }
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
