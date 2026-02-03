use std::hash::Hash;
use std::num::NonZeroUsize;

#[derive(Debug)]
pub(crate) struct TinyLruCache<K, V> {
    cap: NonZeroUsize,
    entries: Vec<(K, V)>,
}

impl<K: Eq + Hash, V> TinyLruCache<K, V> {
    pub(crate) fn new(cap: NonZeroUsize) -> Self {
        Self {
            cap,
            entries: Vec::with_capacity(cap.get()),
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn get(&mut self, key: &K) -> Option<&V> {
        let pos = self.entries.iter().position(|(k, _)| k == key)?;
        if pos != 0 {
            let entry = self.entries.remove(pos);
            self.entries.insert(0, entry);
        }
        self.entries.first().map(|(_, value)| value)
    }

    pub(crate) fn put(&mut self, key: K, value: V) -> Option<V> {
        let mut old_value = None;
        if let Some(pos) = self.entries.iter().position(|(k, _)| k == &key) {
            let (_, existing) = self.entries.remove(pos);
            old_value = Some(existing);
        } else if self.entries.len() == self.cap.get() {
            self.entries.pop();
        }
        self.entries.insert(0, (key, value));
        old_value
    }

    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}

#[cfg(test)]
mod tests {
    use super::TinyLruCache;
    use std::num::NonZeroUsize;

    fn cache(cap: usize) -> TinyLruCache<&'static str, i32> {
        TinyLruCache::new(NonZeroUsize::new(cap).unwrap())
    }

    #[test]
    fn get_on_empty_returns_none() {
        let mut cache = cache(2);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn put_then_get_hits() {
        let mut cache = cache(2);
        assert_eq!(cache.put("a", 1), None);
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn put_existing_key_returns_old_value_and_updates() {
        let mut cache = cache(2);
        assert_eq!(cache.put("a", 1), None);
        assert_eq!(cache.put("a", 9), Some(1));
        assert_eq!(cache.get(&"a"), Some(&9));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn evicts_lru_when_capacity_exceeded() {
        let mut cache = cache(2);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn get_promotes_to_mru_affecting_eviction() {
        let mut cache = cache(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);
        assert_eq!(cache.get(&"a"), Some(&1));
        cache.put("d", 4);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"c"), Some(&3));
        assert_eq!(cache.get(&"d"), Some(&4));
    }

    #[test]
    fn put_counts_as_use_promotes_to_mru() {
        let mut cache = cache(2);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("a", 3);
        cache.put("c", 4);
        assert_eq!(cache.get(&"b"), None);
        assert_eq!(cache.get(&"a"), Some(&3));
        assert_eq!(cache.get(&"c"), Some(&4));
    }

    #[test]
    fn get_miss_does_not_change_order() {
        let mut cache = cache(2);
        cache.put("a", 1);
        cache.put("b", 2);
        assert_eq!(cache.get(&"zzz"), None);
        cache.put("c", 3);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"c"), Some(&3));
    }

    #[test]
    fn repeated_gets_do_not_break_invariants() {
        let mut cache = cache(4);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);
        cache.put("d", 4);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"d"), Some(&4));
        cache.put("e", 5);

        assert_eq!(cache.get(&"c"), None);
        assert_eq!(cache.get(&"e"), Some(&5));
        assert_eq!(cache.get(&"d"), Some(&4));
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(cache.get(&"b"), Some(&2));
    }

    #[test]
    fn capacity_one_always_keeps_last_used() {
        let mut cache = cache(1);
        cache.put("a", 1);
        assert_eq!(cache.get(&"a"), Some(&1));
        cache.put("b", 2);
        assert_eq!(cache.get(&"a"), None);
        assert_eq!(cache.get(&"b"), Some(&2));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn iter_is_mru_order() {
        let mut cache = cache(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);
        let keys: Vec<&str> = cache.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["c", "b", "a"]);
        cache.get(&"a");
        let keys: Vec<&str> = cache.iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, vec!["a", "c", "b"]);
    }
}

#[cfg(all(test, feature = "custom-lru", feature = "lru-crate"))]
mod parity_tests {
    use super::TinyLruCache;
    use lru::LruCache;
    use std::num::NonZeroUsize;

    fn iter_snapshot<K: Copy + Eq + std::hash::Hash, V: Copy>(
        cache: &TinyLruCache<K, V>,
    ) -> Vec<(K, V)> {
        cache.iter().map(|(k, v)| (*k, *v)).collect()
    }

    fn iter_snapshot_lru<K: Copy + Eq + std::hash::Hash, V: Copy>(
        cache: &LruCache<K, V>,
    ) -> Vec<(K, V)> {
        cache.iter().map(|(k, v)| (*k, *v)).collect()
    }

    #[test]
    fn parity_matches_lru_crate_example_sequence() {
        let mut tiny = TinyLruCache::new(NonZeroUsize::new(2).unwrap());
        let mut lru = LruCache::new(NonZeroUsize::new(2).unwrap());

        assert_eq!(tiny.put("apple", 3), lru.put("apple", 3));
        assert_eq!(tiny.put("banana", 2), lru.put("banana", 2));
        assert_eq!(tiny.get(&"apple").copied(), lru.get(&"apple").copied());
        assert_eq!(tiny.get(&"banana").copied(), lru.get(&"banana").copied());
        assert_eq!(tiny.get(&"pear").copied(), lru.get(&"pear").copied());
        assert_eq!(tiny.put("banana", 4), lru.put("banana", 4));
        assert_eq!(tiny.put("pear", 5), lru.put("pear", 5));
        assert_eq!(tiny.get(&"pear").copied(), lru.get(&"pear").copied());
        assert_eq!(tiny.get(&"banana").copied(), lru.get(&"banana").copied());
        assert_eq!(tiny.get(&"apple").copied(), lru.get(&"apple").copied());

        assert_eq!(iter_snapshot(&tiny), iter_snapshot_lru(&lru));
    }

    #[test]
    fn parity_random_operation_trace_small_keyspace() {
        let mut tiny: TinyLruCache<u8, u16> = TinyLruCache::new(NonZeroUsize::new(4).unwrap());
        let mut lru: LruCache<u8, u16> = LruCache::new(NonZeroUsize::new(4).unwrap());

        let mut state: u64 = 0x1234_5678_9abc_def0;
        let mut next_u64 = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };

        for _ in 0..1000 {
            let roll = (next_u64() % 100) as u8;
            let key = (next_u64() % 9) as u8;
            if roll < 70 {
                let tiny_val = tiny.get(&key).copied();
                let lru_val = lru.get(&key).copied();
                assert_eq!(tiny_val, lru_val);
            } else {
                let value = (next_u64() & 0xffff) as u16;
                let tiny_old = tiny.put(key, value);
                let lru_old = lru.put(key, value);
                assert_eq!(tiny_old, lru_old);
            }

            assert_eq!(tiny.len(), lru.len());
            assert_eq!(iter_snapshot(&tiny), iter_snapshot_lru(&lru));
        }
    }

    #[test]
    fn parity_update_existing_key_does_not_evict_unnecessarily() {
        let mut tiny = TinyLruCache::new(NonZeroUsize::new(2).unwrap());
        let mut lru = LruCache::new(NonZeroUsize::new(2).unwrap());

        assert_eq!(tiny.put(1, 10), lru.put(1, 10));
        assert_eq!(tiny.put(2, 20), lru.put(2, 20));
        assert_eq!(tiny.put(2, 21), lru.put(2, 21));
        assert_eq!(tiny.put(3, 30), lru.put(3, 30));

        let tiny_items = iter_snapshot(&tiny);
        let lru_items = iter_snapshot_lru(&lru);
        assert_eq!(tiny_items, lru_items);
        assert_eq!(tiny_items, vec![(3, 30), (2, 21)]);
    }
}
