//! AgentLang runtime prelude.
//!
//! These functions are emitted into generated Rust source to implement
//! AgentLang semantics that Rust doesn't provide directly:
//! bounds-checked indexing, map access, etc.

use std::collections::HashMap;

/// Bounds-checked read of a `Vec<T>` using an `i32` index.
///
/// Panics with an AgentLang-style message if the index is out of bounds.
pub fn al_index<T>(arr: &Vec<T>, idx: i32) -> &T {
    if idx < 0 || idx as usize >= arr.len() {
        panic!(
            "AgentLang: index out of bounds: {} (length: {})",
            idx,
            arr.len()
        );
    }
    &arr[idx as usize]
}

/// Bounds-checked mutable read of a `Vec<T>` using an `i32` index.
pub fn al_index_mut<T>(arr: &mut Vec<T>, idx: i32) -> &mut T {
    if idx < 0 || idx as usize >= arr.len() {
        panic!(
            "AgentLang: index out of bounds: {} (length: {})",
            idx,
            arr.len()
        );
    }
    &mut arr[idx as usize]
}

/// Look up a key in a `HashMap<K, V>`. Returns `None` if absent.
pub fn al_map_get<'a, K, V>(map: &'a HashMap<K, V>, key: &K) -> Option<&'a V>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    map.get(key)
}

/// Insert a key-value pair into a `HashMap<K, V>`.
pub fn al_map_insert<K, V>(map: &mut HashMap<K, V>, key: K, val: V) -> Option<V>
where
    K: std::cmp::Eq + std::hash::Hash,
{
    map.insert(key, val)
}
