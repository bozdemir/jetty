//! Pure tab-transfer + eligibility logic for tab detach/reattach. GPU/window
//! plumbing lives in `app.rs`; everything here is unit-testable without an
//! event loop.

/// A tab may be detached only when the main window keeps at least one tab.
pub fn can_detach(main_tab_count: usize) -> bool {
    main_tab_count >= 2
}

/// Remove and return the element at `idx`, or `None` if out of range.
/// Generic so this module never needs visibility into `Tab`'s fields.
pub fn take_tab<T>(v: &mut Vec<T>, idx: usize) -> Option<T> {
    if idx < v.len() {
        Some(v.remove(idx))
    } else {
        None
    }
}

/// Active index after a reattached tab is appended to a vec whose length is now
/// `tabs_len_after_push`.
pub fn reattach_index(tabs_len_after_push: usize) -> usize {
    tabs_len_after_push.saturating_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detach_requires_at_least_two_tabs() {
        assert!(!can_detach(0));
        assert!(!can_detach(1));
        assert!(can_detach(2));
        assert!(can_detach(5));
    }

    #[test]
    fn take_tab_removes_and_returns_in_range() {
        let mut v = vec!['a', 'b', 'c'];
        assert_eq!(take_tab(&mut v, 1), Some('b'));
        assert_eq!(v, vec!['a', 'c']);
    }

    #[test]
    fn take_tab_out_of_range_is_none_and_no_mutation() {
        let mut v = vec!['a'];
        assert_eq!(take_tab(&mut v, 5), None);
        assert_eq!(v, vec!['a']);
    }

    #[test]
    fn reattached_tab_becomes_active_last() {
        // after pushing onto a vec that now has length 3, active index is 2
        assert_eq!(reattach_index(3), 2);
    }
}
