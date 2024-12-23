use crate::RespFrame;
use dashmap::DashMap;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Backend(Arc<BackendInner>);

#[derive(Debug)]
pub struct BackendInner {
    pub(crate) map: DashMap<String, RespFrame>,
    pub(crate) hmap: DashMap<String, DashMap<String, RespFrame>>,
}

impl Deref for Backend {
    type Target = BackendInner;
    /// Deref implementation for Backend.
    ///
    /// This will return a reference to BackendInner, which allows you to call all the methods on
    /// BackendInner on the Backend struct.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self(Arc::new(BackendInner::default()))
    }
}
impl Default for BackendInner {
    fn default() -> Self {
        Self {
            map: DashMap::new(),
            hmap: DashMap::new(),
        }
    }
}
impl Backend {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a value from the map.
    ///
    /// The value is retrieved from the map with the given key.
    /// If the key is not found, `None` is returned.
    pub fn get(&self, key: &str) -> Option<RespFrame> {
        self.map.get(key).map(|v| v.value().clone())
    }

    /// Stores a value in the map associated with the given key.
    ///
    /// If a value already exists for the given key, it is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying where the value is to be stored in the map.
    /// * `value` - The value to be stored in the map.
    pub fn set(&self, key: String, value: RespFrame) {
        self.map.insert(key, value);
    }

    /// Get a value from the hash map.
    ///
    /// This function retrieves the value associated with the given field within the hash map
    /// identified by the provided key. If both the key and field are found, the corresponding
    /// value is returned as a `RespFrame`. If the key or field is not found, `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the hash map.
    /// * `field` - The field within the hash map whose value is to be retrieved.
    ///
    /// # Returns
    ///
    /// An `Option<RespFrame>` containing the value if it exists, or `None` if either the key or
    /// field is not present.
    pub fn hget(&self, key: &str, field: &str) -> Option<RespFrame> {
        self.hmap
            .get(key)
            .and_then(|v| v.get(field).map(|v| v.value().clone()))
    }

    /// Stores a value in the hash map identified by the given key.
    ///
    /// The value is associated with the given field within the hash map.
    /// If the key is not found, a new hash map is created and the value is stored. If the field is
    /// not found, it is created and the value is stored.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the hash map in which the value is to be stored.
    /// * `field` - The field within the hash map with which the value is to be associated.
    /// * `value` - The value to be stored in the hash map.
    pub fn hset(&self, key: String, field: String, value: RespFrame) {
        let hmap = self.hmap.entry(key).or_default();
        hmap.insert(field, value);
    }

    /// Retrieves all the key-value pairs in the hash map identified by the given key.
    ///
    /// If the key is found, the hash map is cloned and returned. If the key is not found,
    /// `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `key` - The key identifying the hash map from which all key-value pairs are to be
    ///   retrieved.
    ///
    /// # Returns
    ///
    /// An `Option<DashMap<String, RespFrame>>` containing the hash map if it exists, or
    /// `None` if the key is not present.
    pub fn hgetall(&self, key: &str) -> Option<DashMap<String, RespFrame>> {
        self.hmap.get(key).map(|v| v.clone())
    }
}
