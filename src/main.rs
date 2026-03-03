use core::ptr;
use std::collections::HashMap;

fn main() {
    let mut lru = Lru::new(2);

    lru.put(42, "hello".to_string());
    lru.put(1, "sup?".to_string());
    lru.put(2, "sup?".to_string());

    println!("{}", lru.get(42).is_none());
}


// data: map[key => (value, list item)]
// seq: linked list [key]

// note on api: 
// 1. put(key, val) -> (
//  option<val> (return overwritten value if any),
//  vec<(key, val)> (return evicted entries if any)
// )
// 2. ctor that defines the limit

pub type Int = i64;

#[derive(Clone, Copy)]
struct Node {
    key: Int,
    prev: *mut Self,
    next: *mut Self,
}

struct Entry {
    value: String,
    node: *mut Node,
}

pub struct Lru {
    data: HashMap<Int, Entry>,
    head: *mut Node,
    tail: *mut Node,
    limit: usize,
}

impl Lru {
    pub fn new(limit: usize) -> Self {
        Self {
            data: Default::default(),
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            limit,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // O(1)
    // get(key): lookup for value, lookup for list item, move list item to back
    pub fn get(&mut self, key: Int) -> Option<String> {
        let Entry { value, node } = self.data.get(&key)?;
        let node = node.clone();
        unsafe {
            if node != self.tail {
                let prev = (*node).prev;
                let next = (*node).next;

                (*prev).next = next;
                (*next).prev = prev;

                if node == self.head {
                    self.head = next;
                }

                (*(self.tail)).next = node;
                self.tail = node;                
            }
        }
        Some(value.to_string())
    }

    // O(1)
    // put(key, val): 
    // update (key exists) - in-place value update
    // lookup for list item, move list item to back
    // insert (new key) - check if eviction must be triggerred
    // if yes: remove head from seq, remove key-value from data
    pub fn put(&mut self, key: Int, value: String) {
        if self.limit == 0 {
            return;
        }
        if self.is_empty() {
            let node = Node { key, prev: ptr::null_mut(), next: ptr::null_mut() };
            let mut node = Box::new(node);
            self.head = &mut *node;
            self.tail = &mut *node;
            
            let entry = Entry { value, node: &mut *node };
            self.data.insert(key, entry);
            return;
        }

        let exists = self.data.contains_key(&key);
        if exists {
            let Entry { value: _, node } = self.data.get(&key)
                .expect("existence verified above");
            let node = node.clone();

            unsafe {
                if node != self.tail {
                    let prev = (*node).prev;
                    let next = (*node).next;

                    (*prev).next = next;
                    (*next).prev = prev;

                    if node == self.head {
                        self.head = next;
                    }

                    (*(self.tail)).next = node;
                    self.tail = node;
                }
            }
            self.data.get_mut(&key).expect("existance was verified above").value = value;
        } else {
            // not in the map - check for eviction
            if self.data.len() == self.limit {
                unsafe {
                    let node = *self.head;
                    self.head = (*self.head).next;
                    drop(Box::from(node)); // remove memory allocated for node
                    self.data.remove(&node.key);
                }
            } else {
                let node = Node { key, prev: self.tail, next: ptr::null_mut() };
                let node = &mut *Box::new(node);
                let entry = Entry {
                    value,
                    node,
                };
                self.data.insert(key, entry);

                unsafe {
                    (*self.tail).next = node;
                    self.tail = node;
                }
            }
        }
    }
}

