
use rustc_hash::FxHashMap;
use std::hash::Hash;

/// Union-Find / Disjoint Set data structure for reactive scope inference.
///
/// Uses path compression in `find` and union by rank in `union`
/// for nearly O(1) amortized operations.
pub struct DisjointSet<T: Copy + Eq + Hash> {
    parent: FxHashMap<T, T>,
    rank: FxHashMap<T, u32>,
}

impl<T: Copy + Eq + Hash> Default for DisjointSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Eq + Hash> DisjointSet<T> {
    pub fn new() -> Self {
        Self { parent: FxHashMap::default(), rank: FxHashMap::default() }
    }

    /// Creates a new singleton set containing `item`.
    /// If `item` already exists, this is a no-op.
    pub fn make_set(&mut self, item: T) {
        self.parent.entry(item).or_insert(item);
        self.rank.entry(item).or_insert(0);
    }

    /// Returns the representative of the set containing `item`,
    /// applying path compression along the way.
    ///
    /// Returns `None` if `item` has not been registered via `make_set`.
    pub fn find(&mut self, item: T) -> Option<T> {
        let p = *self.parent.get(&item)?;
        if p == item {
            return Some(item);
        }
        let root = self.find(p)?;
        self.parent.insert(item, root);
        Some(root)
    }

    /// Merges the sets containing `a` and `b` using union by rank.
    ///
    /// Returns `None` if either `a` or `b` has not been registered via `make_set`.
    pub fn union(&mut self, a: T, b: T) -> Option<()> {
        let root_a = self.find(a)?;
        let root_b = self.find(b)?;
        if root_a == root_b {
            return Some(());
        }

        let rank_a = self.rank[&root_a];
        let rank_b = self.rank[&root_b];

        if rank_a < rank_b {
            self.parent.insert(root_a, root_b);
        } else if rank_a > rank_b {
            self.parent.insert(root_b, root_a);
        } else {
            self.parent.insert(root_b, root_a);
            *self.rank.get_mut(&root_a).unwrap() += 1;
        }
        Some(())
    }

    /// Returns `true` if `a` and `b` belong to the same set.
    ///
    /// Returns `None` if either `a` or `b` has not been registered via `make_set`.
    pub fn same_set(&mut self, a: T, b: T) -> Option<bool> {
        Some(self.find(a)? == self.find(b)?)
    }

    /// Returns all sets grouped by their representative element.
    pub fn sets(&mut self) -> FxHashMap<T, Vec<T>> {
        let items: Vec<T> = self.parent.keys().copied().collect();
        let mut result: FxHashMap<T, Vec<T>> = FxHashMap::default();
        for item in items {
            if let Some(root) = self.find(item) {
                result.entry(root).or_default().push(item);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_singleton_sets() {
        let mut ds = DisjointSet::new();
        ds.make_set(1);
        ds.make_set(2);
        assert_eq!(ds.find(1), Some(1));
        assert_eq!(ds.find(2), Some(2));
        assert_eq!(ds.same_set(1, 2), Some(false));
    }

    #[test]
    fn test_find_unregistered_returns_none() {
        let mut ds: DisjointSet<i32> = DisjointSet::new();
        assert_eq!(ds.find(42), None);
    }

    #[test]
    fn test_union_unregistered_returns_none() {
        let mut ds = DisjointSet::new();
        ds.make_set(1);
        assert_eq!(ds.union(1, 99), None);
        assert_eq!(ds.union(99, 1), None);
    }

    #[test]
    fn test_same_set_unregistered_returns_none() {
        let mut ds = DisjointSet::new();
        ds.make_set(1);
        assert_eq!(ds.same_set(1, 99), None);
    }

    #[test]
    fn test_union_and_find() {
        let mut ds = DisjointSet::new();
        ds.make_set(1);
        ds.make_set(2);
        ds.make_set(3);

        ds.union(1, 2);
        assert_eq!(ds.same_set(1, 2), Some(true));
        assert_eq!(ds.same_set(1, 3), Some(false));

        ds.union(2, 3);
        assert_eq!(ds.same_set(1, 3), Some(true));
    }

    #[test]
    fn test_sets() {
        let mut ds = DisjointSet::new();
        for i in 0..6 {
            ds.make_set(i);
        }
        ds.union(0, 1);
        ds.union(1, 2);
        ds.union(3, 4);

        let groups = ds.sets();
        assert_eq!(groups.len(), 3); // {0,1,2}, {3,4}, {5}
    }

    #[test]
    fn test_union_idempotent() {
        let mut ds = DisjointSet::new();
        ds.make_set(1);
        ds.make_set(2);
        let _ = ds.union(1, 2);
        let _ = ds.union(1, 2);
        assert_eq!(ds.same_set(1, 2), Some(true));
        assert_eq!(ds.sets().len(), 1);
    }
}
