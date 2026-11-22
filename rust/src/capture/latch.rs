//! An immutable frame selected against the shortcut time, not the OCR time.
#[derive(Clone)]
pub struct Frame<K, T> {
    pub key: K,
    pub alt_started: i64,
    pub presented: i64,
    pub value: T,
}
pub fn select<K: PartialEq, T: Clone>(
    frame: Option<&Frame<K, T>>,
    key: &K,
    alt_started: i64,
    pressed: i64,
    maximum_age: i64,
) -> Option<T> {
    let frame = frame?;
    (frame.key == *key
        && alt_started > 0
        && frame.alt_started == alt_started
        && frame.presented >= alt_started
        && frame.presented <= pressed
        && pressed - frame.presented <= maximum_age)
        .then(|| frame.value.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    #[test]
    fn releasing_alt_and_replacing_the_live_frame_cannot_change_a_latched_image() {
        let mut live = Some(Frame {
            key: 7,
            alt_started: 100,
            presented: 120,
            value: Arc::new(vec![1, 2, 3]),
        });
        let frozen = select(live.as_ref(), &7, 100, 130, 50).unwrap();
        live = None; // key-up clears the live cache
        assert!(select(live.as_ref(), &7, 100, 130, 50).is_none());
        live = Some(Frame {
            key: 7,
            alt_started: 200,
            presented: 220,
            value: Arc::new(vec![9]),
        });
        assert_eq!(&*frozen, &[1, 2, 3]);
        assert!(select(live.as_ref(), &7, 100, 130, 50).is_none());
    }
    #[test]
    fn a_frame_from_before_alt_after_the_shortcut_or_another_target_is_rejected() {
        let mut frame = Frame {
            key: 7,
            alt_started: 100,
            presented: 120,
            value: 42,
        };
        assert_eq!(select(Some(&frame), &7, 100, 130, 50), Some(42));
        assert_eq!(select(Some(&frame), &8, 100, 130, 50), None);
        assert_eq!(select(Some(&frame), &7, 100, 110, 50), None);
        assert_eq!(select(Some(&frame), &7, 100, 200, 50), None);
        assert_eq!(select(Some(&frame), &7, 0, 130, 50), None);
        frame.presented = 90;
        assert_eq!(select(Some(&frame), &7, 100, 130, 50), None);
        frame.presented = 120;
        assert_eq!(select(Some(&frame), &7, 110, 130, 50), None);
    }
}
