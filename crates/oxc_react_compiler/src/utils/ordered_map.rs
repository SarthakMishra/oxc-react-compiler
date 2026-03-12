
use std::ops::Index;

/// Insertion-ordered map, used throughout the compiler pipeline.
///
/// Backed by a `Vec<(K, V)>` to maintain insertion order while
/// providing a map-like API. Best suited for small to medium
/// collections where insertion order matters.
#[derive(Debug, Clone)]
pub struct OrderedMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K: Eq, V> OrderedMap<K, V> {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { entries: Vec::with_capacity(cap) }
    }

    /// Inserts a key-value pair. If the key already exists, its value is updated.
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(entry) = self.entries.iter_mut().find(|(k, _)| k == &key) {
            entry.1 = value;
        } else {
            self.entries.push((key, value));
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.entries.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.iter().any(|(k, _)| k == key)
    }

    /// Removes the entry with the given key, returning its value if found.
    /// Preserves the relative order of remaining elements.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let pos = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(pos).1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.entries.iter_mut().map(|(k, v)| (&*k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }
}

impl<K: Eq, V> Default for OrderedMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

/// # Panics
///
/// Panics if the key is not present. Prefer `OrderedMap::get()` when key
/// existence cannot be guaranteed at the call-site.
///
/// # Safety audit (2026-03-12)
///
/// Currently `OrderedMap` has **no external callers** — it is only exercised by
/// the unit tests in this module (`test_index` and `test_index_missing_key_panics`).
/// The `Index` impl is provided for API completeness and ergonomics; callers that
/// cannot guarantee key presence should use `OrderedMap::get()` instead.
impl<K: Eq, V> Index<&K> for OrderedMap<K, V> {
    type Output = V;

    fn index(&self, key: &K) -> &V {
        self.get(key).expect("key not found in OrderedMap")
    }
}

impl<K: Eq, V> IntoIterator for OrderedMap<K, V> {
    type Item = (K, V);
    type IntoIter = std::vec::IntoIter<(K, V)>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(map.get(&"a"), Some(&1));
        assert_eq!(map.get(&"b"), Some(&2));
        assert_eq!(map.get(&"c"), None);
    }

    #[test]
    fn test_insert_updates_existing() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        map.insert("a", 42);
        assert_eq!(map.get(&"a"), Some(&42));
        assert_eq!(map.len(), 1);
    }

    #[test]
    fn test_preserves_insertion_order() {
        let mut map = OrderedMap::new();
        map.insert(3, "c");
        map.insert(1, "a");
        map.insert(2, "b");
        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys, vec![3, 1, 2]);
    }

    #[test]
    fn test_remove() {
        let mut map = OrderedMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        map.insert("c", 3);
        assert_eq!(map.remove(&"b"), Some(2));
        assert_eq!(map.len(), 2);
        let keys: Vec<_> = map.keys().copied().collect();
        assert_eq!(keys, vec!["a", "c"]);
    }

    #[test]
    fn test_index() {
        let mut map = OrderedMap::new();
        map.insert("x", 99);
        assert_eq!(map[&"x"], 99);
    }

    #[test]
    #[should_panic(expected = "key not found")]
    fn test_index_missing_key_panics() {
        let map: OrderedMap<&str, i32> = OrderedMap::new();
        let _ = map[&"missing"];
    }

    #[test]
    fn test_into_iter() {
        let mut map = OrderedMap::new();
        map.insert(1, "a");
        map.insert(2, "b");
        let collected: Vec<_> = map.into_iter().collect();
        assert_eq!(collected, vec![(1, "a"), (2, "b")]);
    }

    #[test]
    fn test_default() {
        let map: OrderedMap<String, i32> = OrderedMap::default();
        assert!(map.is_empty());
    }
}
