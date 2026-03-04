use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::hash::Hash;

#[derive(Clone)]
struct Entry<V: Clone> {
    value: V,
    seq: u64,
}

struct SeqKey<K> {
    seq: u64,
    key: K,
}

impl<K> PartialEq for SeqKey<K> {
    fn eq(&self, other: &Self) -> bool {
        self.seq == other.seq
    }
}

impl<K> Eq for SeqKey<K> {}

impl<K> PartialOrd for SeqKey<K> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<K> Ord for SeqKey<K> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.seq.cmp(&other.seq)
    }
}

pub struct Lru<K: Clone + Eq + Hash, V: Clone> {
    limit: usize,
    data: HashMap<K, Entry<V>>,
    lru: BinaryHeap<Reverse<SeqKey<K>>>,
    seq: u64,
}

impl<K: Clone + Eq + Hash, V: Clone> Lru<K, V> {
    pub fn new(limit: usize) -> Self {
        Self {
            limit,
            data: Default::default(),
            lru: BinaryHeap::new(),
            seq: 0,
        }
    }

    fn evict_one(&mut self) -> Option<(K, V)> {
        while let Some(Reverse(sk)) = self.lru.pop() {
            if let Some(entry) = self.data.get(&sk.key)
                && entry.seq == sk.seq
            {
                return self.data.remove(&sk.key).map(|e| (sk.key, e.value));
            }
        }
        None
    }

    pub fn get(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.data.get_mut(key) {
            let value = entry.value.clone();
            self.seq += 1;
            entry.seq = self.seq;
            self.lru.push(Reverse(SeqKey {
                seq: self.seq,
                key: key.clone(),
            }));
            Some(value)
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) -> (Option<V>, Option<(K, V)>) {
        if self.limit == 0 {
            return (None, None);
        }
        if let Some(entry) = self.data.get_mut(&key) {
            let old_value = std::mem::replace(&mut entry.value, value);
            self.seq += 1;
            entry.seq = self.seq;
            self.lru.push(Reverse(SeqKey { seq: self.seq, key }));
            (Some(old_value), None)
        } else {
            let evicted = if self.data.len() >= self.limit {
                self.evict_one()
            } else {
                None
            };
            self.seq += 1;
            self.lru.push(Reverse(SeqKey {
                seq: self.seq,
                key: key.clone(),
            }));
            self.data.insert(
                key,
                Entry {
                    value,
                    seq: self.seq,
                },
            );
            (None, evicted)
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
