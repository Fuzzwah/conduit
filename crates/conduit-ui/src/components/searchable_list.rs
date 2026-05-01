//! Shared state for searchable, scrollable lists.

use super::TextInputState;

#[derive(Debug, Clone)]
pub struct SearchableListState {
    /// Search/filter input
    pub search: TextInputState,
    /// Indices of items matching the filter
    pub filtered: Vec<usize>,
    /// Currently selected index in the filtered list
    pub selected: usize,
    /// Maximum visible items in the list
    pub max_visible: usize,
    /// Scroll offset for the list
    pub scroll_offset: usize,
}

impl SearchableListState {
    pub fn new(max_visible: usize) -> Self {
        Self {
            search: TextInputState::new(),
            filtered: Vec::new(),
            selected: 0,
            max_visible,
            scroll_offset: 0,
        }
    }

    pub fn reset(&mut self) {
        self.search.clear();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn set_filtered(&mut self, filtered: Vec<usize>) {
        self.filtered = filtered;
        self.clamp_selection();
        self.scroll_offset = 0;
    }

    pub fn clamp_selection(&mut self) {
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    /// Select previous item.
    pub fn select_prev(&mut self) {
        if !self.filtered.is_empty() && self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    /// Select next item.
    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() && self.selected < self.filtered.len() - 1 {
            self.selected += 1;
            if self.selected >= self.scroll_offset + self.max_visible {
                self.scroll_offset = self.selected - self.max_visible + 1;
            }
        }
    }

    /// Page up (move up by visible count).
    pub fn page_up(&mut self) {
        if !self.filtered.is_empty() {
            let page_size = self.max_visible;
            if self.selected >= page_size {
                self.selected -= page_size;
            } else {
                self.selected = 0;
            }
            if self.selected < self.scroll_offset {
                self.scroll_offset = self.selected;
            }
        }
    }

    /// Page down (move down by visible count).
    pub fn page_down(&mut self) {
        if !self.filtered.is_empty() {
            let page_size = self.max_visible;
            let max_idx = self.filtered.len().saturating_sub(1);
            if self.selected + page_size <= max_idx {
                self.selected += page_size;
            } else {
                self.selected = max_idx;
            }
            if self.selected >= self.scroll_offset + self.max_visible {
                self.scroll_offset = self.selected.saturating_sub(self.max_visible - 1);
            }
        }
    }

    /// Select item at a given visual row (for mouse clicks).
    /// Returns true if an item was selected.
    pub fn select_at_row(&mut self, row: usize) -> bool {
        let target_idx = self.scroll_offset + row;
        if target_idx < self.filtered.len() {
            self.selected = target_idx;
            true
        } else {
            false
        }
    }

    pub fn visible_len(&self) -> usize {
        self.max_visible.min(self.filtered.len().max(1))
    }
}

impl Default for SearchableListState {
    fn default() -> Self {
        Self::new(10)
    }
}

/// Return indices into `items` whose `key(item)` contains `query` (case-insensitive).
/// An empty query returns every index in original order.
pub fn filter_indices<T, F>(items: &[T], query: &str, key: F) -> Vec<usize>
where
    F: Fn(&T) -> &str,
{
    if query.is_empty() {
        return (0..items.len()).collect();
    }
    let needle = query.to_lowercase();
    items
        .iter()
        .enumerate()
        .filter(|(_, it)| key(it).to_lowercase().contains(&needle))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::filter_indices;

    #[test]
    fn empty_query_returns_all_indices_in_order() {
        let items = vec!["alpha", "beta", "gamma"];
        assert_eq!(filter_indices(&items, "", |s| s), vec![0, 1, 2]);
    }

    #[test]
    fn case_insensitive_substring_match() {
        let items = vec!["Apple", "Banana", "applet", "PEAR"];
        assert_eq!(filter_indices(&items, "app", |s| s), vec![0, 2]);
        assert_eq!(filter_indices(&items, "AN", |s| s), vec![1]);
        assert_eq!(filter_indices(&items, "ear", |s| s), vec![3]);
    }

    #[test]
    fn no_matches_returns_empty() {
        let items = vec!["alpha", "beta"];
        assert!(filter_indices(&items, "zzz", |s| s).is_empty());
    }

    #[test]
    fn key_extractor_is_used() {
        let items = vec![("a", "first"), ("b", "second"), ("c", "third")];
        // Filter on the second tuple element.
        assert_eq!(filter_indices(&items, "ir", |(_, v)| *v), vec![0, 2]);
    }
}
