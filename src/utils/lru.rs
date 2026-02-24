// Port of go-trafilatura/internal/lru/cache.go

use std::collections::HashMap;

/// Simple FIFO-eviction cache mapping strings to integer counts.
///
/// Used for text deduplication: tracks how many times a given text string
/// has been seen. When capacity is exceeded, the oldest entry is evicted.
///
/// Note: despite the name "LRU", the Go implementation uses FIFO eviction
/// (oldest-inserted, not least-recently-used). We match that behavior exactly.
pub struct LruCache {
    max_size: usize,
    /// Insertion-ordered list of keys (oldest first).
    keys: Vec<String>,
    data: HashMap<String, usize>,
}

impl LruCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            keys: Vec::new(),
            data: HashMap::new(),
        }
    }

    /// Fetch the count for a key, returning None if not present.
    ///
    /// Port of `Cache.Get`.
    pub fn get(&self, key: &str) -> Option<usize> {
        self.data.get(key).copied()
    }

    /// Remove a single entry from the cache.
    ///
    /// Port of `Cache.Remove`.
    pub fn remove(&mut self, key: &str) {
        if self.data.remove(key).is_some() {
            self.keys.retain(|k| k != key);
        }
    }

    /// Store a value in the cache, evicting the oldest entry if full.
    ///
    /// Port of `Cache.Put`.
    pub fn put(&mut self, key: String, value: usize) {
        if let Some(v) = self.data.get_mut(&key) {
            // Key exists: update in-place, no eviction or key-list change needed.
            *v = value;
            return;
        }

        // Evict oldest if at capacity.
        if self.max_size > 0 && self.keys.len() >= self.max_size {
            let oldest = self.keys.remove(0);
            self.data.remove(&oldest);
        }

        self.keys.push(key.clone());
        self.data.insert(key, value);
    }

    /// Remove all entries from the cache.
    ///
    /// Port of `Cache.Clear`.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.data.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_put_get() {
        let mut cache = LruCache::new(10);
        cache.put("hello".to_string(), 1);
        assert_eq!(cache.get("hello"), Some(1));
        assert_eq!(cache.get("missing"), None);
    }

    #[test]
    fn test_update_existing_key() {
        let mut cache = LruCache::new(10);
        cache.put("key".to_string(), 1);
        cache.put("key".to_string(), 5);
        assert_eq!(cache.get("key"), Some(5));
        // Should not have duplicate keys.
        assert_eq!(cache.keys.len(), 1);
    }

    #[test]
    fn test_fifo_eviction() {
        let mut cache = LruCache::new(3);
        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);
        cache.put("c".to_string(), 3);
        // At capacity; inserting "d" should evict "a" (oldest).
        cache.put("d".to_string(), 4);
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), Some(2));
        assert_eq!(cache.get("c"), Some(3));
        assert_eq!(cache.get("d"), Some(4));
    }

    #[test]
    fn test_remove() {
        let mut cache = LruCache::new(10);
        cache.put("x".to_string(), 99);
        cache.remove("x");
        assert_eq!(cache.get("x"), None);
        // Removing a non-existent key is a no-op.
        cache.remove("nonexistent");
    }

    #[test]
    fn test_clear() {
        let mut cache = LruCache::new(10);
        cache.put("a".to_string(), 1);
        cache.put("b".to_string(), 2);
        cache.clear();
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), None);
        assert!(cache.keys.is_empty());
    }

    #[test]
    fn test_duplicate_count_pattern() {
        // This is the pattern used in duplicateTest:
        // get count, then put count+1.
        let mut cache = LruCache::new(100);
        let text = "Some test string".to_string();

        let count = cache.get(&text).unwrap_or(0);
        assert_eq!(count, 0);
        cache.put(text.clone(), count + 1);

        let count = cache.get(&text).unwrap_or(0);
        assert_eq!(count, 1);
        cache.put(text.clone(), count + 1);

        assert_eq!(cache.get(&text), Some(2));
    }

    #[test]
    fn test_zero_capacity() {
        // max_size=0 means no eviction (unlimited). Match Go: if maxSize=0,
        // the check `len(c.keys) >= c.maxSize` is always true, but Go uses
        // `c.maxSize > 0` guard — so zero maxSize means no eviction.
        let mut cache = LruCache::new(0);
        for i in 0..100 {
            cache.put(i.to_string(), i);
        }
        assert_eq!(cache.data.len(), 100);
    }
}
