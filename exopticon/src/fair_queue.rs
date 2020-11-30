/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::collections::{HashMap, VecDeque};

/// The `FairQueue` queues values identified by a unique key. When a
/// value is pushed when a key already present in the queue, the old
/// value is replaced.
pub struct FairQueue<K, V> {
    /// Queue of keys
    pub queue: VecDeque<K>,
    /// HashMap of values
    pub items: HashMap<K, V>,
}

impl<K, V> FairQueue<K, V>
where
    K: std::cmp::Eq + std::hash::Hash + std::clone::Clone,
{
    /// Returns initialized `FairQueue`
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            items: HashMap::new(),
        }
    }

    /// Returns number of items in queue
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Removes item with given key from queue.
    pub fn remove(&mut self, key: &K) {
        let mut new_queue = VecDeque::new();
        for i in &self.queue {
            if i != key {
                new_queue.push_back(i.clone());
            }
        }
        self.queue = new_queue;
        self.items.remove(key);
    }

    /// Pushes an item on the queue. If an item with the given key
    /// already exists, it is replaced.
    pub fn push_back(&mut self, key: K, item: V) {
        if self.items.insert(key.clone(), item).is_none() {
            self.queue.push_back(key);
        }
    }

    /// Pop an item from the front of the queue. Returns `None` if the
    /// queue is empty.
    pub fn pop_front(&mut self) -> Option<V> {
        while let Some(key) = self.queue.pop_front() {
            if let Some(item) = self.items.remove(&key) {
                return Some(item);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len_empty() {
        // Arrange
        let q: FairQueue<u8, &str> = FairQueue::new();

        // Act

        // Assert
        assert_eq!(0, q.len());
    }

    #[test]
    fn test_len_after_replacement() {
        // Arrange
        let mut q: FairQueue<u8, &str> = FairQueue::new();

        // Act
        q.push_back(0, "asdf");
        q.push_back(0, "asdf2");
        q.push_back(1, "asdf3");
        q.push_back(1, "asdf4");

        // Assert
        assert_eq!(2, q.len());
    }

    #[test]
    fn test_len_after_pop() {
        // Arrange
        let mut q: FairQueue<u8, &str> = FairQueue::new();
        q.push_back(0, "asdf");
        q.push_back(1, "qwer");

        // Act
        let len = q.len();
        let val = q.pop_front();
        let len2 = q.len();
        let val2 = q.pop_front();
        let len3 = q.len();
        let val3 = q.pop_front();

        // Assert
        assert_eq!(2, len);
        assert_eq!(Some("asdf"), val);
        assert_eq!(1, len2);
        assert_eq!(Some("qwer"), val2);
        assert_eq!(0, len3);
        assert_eq!(None, val3);
    }

    #[test]
    fn test_pop_fifo() {
        // Arrange
        let mut q: FairQueue<u8, &str> = FairQueue::new();

        q.push_back(0, "asdf");
        q.push_back(1, "qwer");

        // Act
        let val1 = q.pop_front();
        let val2 = q.pop_front();

        // Assert
        assert_eq!(val1, Some("asdf"));
        assert_eq!(val2, Some("qwer"));
    }

    #[test]
    fn test_value_replace() {
        // Arrange
        let mut q: FairQueue<u8, &str> = FairQueue::new();

        q.push_back(0, "asdf");
        q.push_back(1, "QWER");
        q.push_back(0, "zxcv");

        // Act
        let val = q.pop_front();

        //Assert
        assert_eq!(val, Some("zxcv"));
    }

    #[test]
    fn test_key_removal() {
        // Arrange
        let mut q: FairQueue<u8, &str> = FairQueue::new();

        q.push_back(0, "asdf");
        q.push_back(1, "QWER");
        q.push_back(0, "zxcv");

        // Act
        q.remove(&0);
        let val = q.pop_front();

        //Assert
        assert_eq!(val, Some("QWER"));
    }

    #[test]
    fn test_push_after_removal() {
        // Arrange
        let mut q: FairQueue<u8, &str> = FairQueue::new();

        // Act
        q.push_back(0, "asdf");
        q.push_back(1, "zxcv");
        q.remove(&0);
        q.push_back(0, "asdf2");
        let val = q.pop_front();

        // Assert
        assert_eq!(Some("zxcv"), val);
    }
}
