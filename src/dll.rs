//! O(1) LRU cache backed by a HashMap and a doubly-linked list of raw pointers.

use core::ptr;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone)]
struct Node<K: Clone> {
    key: K,
    prev: *mut Self,
    next: *mut Self,
}

#[derive(Clone)]
struct Entry<K: Clone, V: Clone> {
    value: V,
    node: *mut Node<K>,
}

/// LRU cache with O(1) get and put.
///
/// Backed by a [`HashMap`] for fast lookups and a doubly-linked list of raw
/// pointers to track recency. The least-recently-used entry is evicted when
/// the cache reaches its `limit`.
pub struct Lru<K: Clone + Eq + Hash, V: Clone> {
    data: HashMap<K, Entry<K, V>>,
    head: *mut Node<K>,
    tail: *mut Node<K>,
    limit: usize,
}

impl<K: Clone + Eq + Hash, V: Clone> Lru<K, V> {
    /// Create a cache that holds at most `limit` entries.
    pub fn new(limit: usize) -> Self {
        Self {
            data: Default::default(),
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            limit,
        }
    }

    /// True if the cache contains no entries.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Heap-allocate a detached node.
    fn node(key: K) -> *mut Node<K> {
        let node = Node {
            key,
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        };
        Box::into_raw(Box::new(node))
    }

    /// Unlink a node from the list without freeing it.
    fn unlink(&mut self, node: *mut Node<K>) {
        unsafe {
            let prev = (*node).prev;
            let next = (*node).next;
            if !prev.is_null() {
                (*prev).next = next;
            }
            if !next.is_null() {
                (*next).prev = prev;
            }
            if node == self.head {
                self.head = next;
            }
            if node == self.tail {
                self.tail = prev;
            }
        }
    }

    /// Append a node to the tail (most-recently-used end).
    fn push_back(&mut self, node: *mut Node<K>) {
        if self.tail.is_null() {
            self.head = node;
            self.tail = node;
            return;
        }
        if node == self.tail {
            return;
        }
        unsafe {
            let tail = self.tail;
            (*node).prev = tail;
            (*node).next = ptr::null_mut();
            if !tail.is_null() {
                (*tail).next = node;
            }
            self.tail = node;
        }
    }

    /// Pop the head (least-recently-used), free it, return its key.
    fn pull_head(&mut self) -> Option<K> {
        if self.is_empty() || self.head.is_null() {
            return None;
        }
        unsafe {
            let key = (*self.head).key.clone();

            let head = self.head;
            let next = (*head).next;
            if !next.is_null() {
                (*next).prev = ptr::null_mut();
            }
            self.head = next;
            if next.is_null() {
                self.tail = ptr::null_mut();
            }
            drop(Box::from_raw(head));
            Some(key)
        }
    }

    /// Look up a value and promote its key to most-recently-used.
    pub fn get(&mut self, key: &K) -> Option<V> {
        let Entry { value, node } = self.data.get(key).cloned()?;
        if self.tail != node {
            self.unlink(node);
            self.push_back(node);
        }
        Some(value)
    }

    /// Insert or update. Returns (overwritten value, evicted entry).
    pub fn put(&mut self, key: K, value: V) -> (Option<V>, Option<(K, V)>) {
        if self.limit == 0 {
            return (None, None);
        }

        if let Some(entry) = self.data.get_mut(&key) {
            let removed = std::mem::replace(&mut entry.value, value);
            let node = entry.node;
            if self.tail != node {
                self.unlink(node);
                self.push_back(node);
            }
            (Some(removed), None)
        } else {
            let evicted = if self.data.len() >= self.limit {
                let evicted = self
                    .pull_head()
                    .expect("head must exist when cache is full");
                let value = self
                    .data
                    .remove(&evicted)
                    .expect("evicted key must be present")
                    .value;
                Some((evicted, value))
            } else {
                None
            };
            let node = Self::node(key.clone());
            self.data.insert(key, Entry { value, node });
            self.push_back(node);
            (None, evicted)
        }
    }

    /// Keys in eviction order (head = next to be evicted).
    pub fn lru(&self) -> Vec<K> {
        let mut ret = Vec::with_capacity(self.data.len());
        let mut node = self.head;
        while !node.is_null() {
            let Node { key, next, .. } = unsafe { (*node).clone() };
            node = next;
            ret.push(key);
        }
        ret
    }

    /// Snapshot of all cached key-value pairs.
    pub fn data(&self) -> HashMap<K, V> {
        self.data
            .iter()
            .map(|(key, entry)| (key.clone(), entry.value.clone()))
            .collect()
    }
}

impl<K: Clone + Eq + Hash, V: Clone> Drop for Lru<K, V> {
    fn drop(&mut self) {
        let mut node = self.head;
        while !node.is_null() {
            unsafe {
                let next = (*node).next;
                drop(Box::from_raw(node));
                node = next;
            }
        }
    }
}

unsafe impl<K: Clone + Eq + Hash + Send, V: Clone + Send> Send for Lru<K, V> {}
unsafe impl<K: Clone + Eq + Hash + Sync, V: Clone + Sync> Sync for Lru<K, V> {}

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
