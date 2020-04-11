use std::cmp::Ord;
use std::collections::BTreeSet;
use std::iter::IntoIterator;

/// Utility structure for tracking a pool of items and generating events when
/// new items are added or old items are removed.
#[derive(Debug, Clone)]
pub struct ItemPool<T: Ord + Clone> {
    items:       BTreeSet<T>,
    working_set: BTreeSet<T>,
}

impl<T: Ord + Clone> Default for ItemPool<T> {
    fn default() -> Self { Self::new() }
}

impl<T: Ord + Clone> ItemPool<T> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            items:       BTreeSet::new(),
            working_set: BTreeSet::new(),
        }
    }

    /// Updates the internal pool map, returning two vectors of items `(added,
    /// removed)` that represent all new items that were added (items that
    /// appear in the given iterator and not in the previous internal pool)
    /// and all old items that were removed (items that appear in the
    /// previous internal pool but not in the given iterator)
    pub fn update<I>(&mut self, new_target_ids: I) -> (Vec<T>, Vec<T>)
    where
        I: IntoIterator<Item = T>,
    {
        let items = &mut self.items;
        let working_set = &mut self.working_set;

        // Add all new target Ids to the working set
        working_set.extend(new_target_ids);

        let added: Vec<T> = working_set
            .drain_filter(|i| !items.contains(i))
            .collect::<Vec<_>>();
        let removed: Vec<T> = items
            .drain_filter(|i| !working_set.contains(i))
            .collect::<Vec<_>>();

        items.extend(added.iter().cloned());
        working_set.clear();
        (added, removed)
    }
}
