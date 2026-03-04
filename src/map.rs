use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

#[derive(Clone)]
struct Entry<V: Clone> {
    value: V,
    seq: u64,
}

pub struct Lru<K: Clone + Eq + Hash, V: Clone> {
    limit: usize,
    data: HashMap<K, Entry<V>>,
    lru: BTreeMap<u64, K>,
    seq: u64,
}

impl<K: Clone + Eq + Hash, V: Clone> Lru<K, V> {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            data: Default::default(),
            lru: Default::default(),
            seq: 0,
        }
    }

    fn hit(&mut self, key: &K) {
        self.seq += 1;
        if let Some(seq) = self.data.get(key).map(|entry| entry.seq) {
            self.lru.remove(&seq);
        }
        self.lru.insert(self.seq, key.clone());
        if let Some(entry) = self.data.get_mut(key) {
            entry.seq = self.seq;
        }
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.data.get(key).cloned() {
            self.hit(key);
            Some(entry.value)
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) -> (Option<V>, Option<(K, V)>) {
        if self.limit == 0 {
            return (None, None);
        }
        if !self.data.contains_key(&key) {
            let evicted = if self.data.len() == self.limit {
                let (&seq, _) = self.lru.first_key_value().expect("full cache");
                self.lru
                    .remove(&seq)
                    .and_then(|k| self.data.remove(&k).map(|v| (k, v.value)))
            } else {
                None
            };
            self.hit(&key);
            self.data.insert(
                key,
                Entry {
                    value,
                    seq: self.seq,
                },
            );
            (None, evicted)
        } else {
            self.hit(&key);
            let removed = self.data.remove(&key).map(|entry| entry.value.clone());
            self.data.insert(
                key,
                Entry {
                    value,
                    seq: self.seq,
                },
            );
            (removed, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_eviction() {
        let mut lru = Lru::new(2);
        assert_eq!(lru.put(1, "a"), (None, None));
        assert_eq!(lru.put(2, "b"), (None, None));
        assert_eq!(lru.put(3, "c"), (None, Some((1, "a"))));

        assert_eq!(lru.get(&1), None);
        assert_eq!(lru.get(&2), Some("b"));
        assert_eq!(lru.get(&3), Some("c"));
    }

    #[test]
    fn test_usage_eviction() {
        let mut lru = Lru::new(2);
        assert_eq!(lru.put(1, "a"), (None, None));
        assert_eq!(lru.put(2, "b"), (None, None));

        assert_eq!(lru.get(&1), Some("a"));
        assert_eq!(lru.put(3, "c"), (None, Some((2, "b"))));

        assert_eq!(lru.get(&1), Some("a"));
        assert_eq!(lru.get(&2), None);
        assert_eq!(lru.get(&3), Some("c"));
    }
}

/* TODO:

1. Lazy BTreeMap (biggest win)
Don't remove the old entry on get — just insert the new seq. On eviction, walk from the front and skip stale entries (where entry.seq in data doesn't match the seq in lru). Cuts BTreeMap operations per get from 2 to 1. Tradeoff: BTreeMap holds ghost entries (but each key has at most one ghost at a time, so size stays bounded at 2× cache limit).

2. Merge the two HashMap lookups in hit
Currently data.get(key) then data.get_mut(key) — two lookups. A single data.get_mut(key) reads and updates entry.seq in one shot.

3. In-place value update for existing keys in put
data.remove + data.insert causes a reallocation cycle. data.get_mut(key).unwrap().value = value instead.

4. Eliminate contains_key + separate access in put
Use the entry API to avoid the double lookup.

5. Replace BTreeMap with a BinaryHeap + lazy deletion
Same lazy-skip idea as #1 but a binary heap has better cache behaviour for the common case (just peeking/popping the min). Works well if evictions are rare relative to gets.

The combination of #1 + #2 + #3 should close most of the gap with dll.

*/