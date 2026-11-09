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

/// Map a function over an `Option<T>`, producing `Option<U>`.
pub fn al_map_option<T: Clone, U>(opt: &Option<T>, f: impl FnOnce(T) -> U) -> Option<U> {
    opt.as_ref().map(|x| f(x.clone()))
}

/// Map a function over a `Result<T, E>`, producing `Result<U, E>`.
pub fn al_map_result<T: Clone, E: Clone, U>(
    result: &Result<T, E>,
    f: impl FnOnce(T) -> U,
) -> Result<U, E> {
    match result {
        Ok(x) => Ok(f(x.clone())),
        Err(e) => Err(e.clone()),
    }
}

/// Unwrap an `Option<T>`, panicking with an AgentLang-style message if `None`.
pub fn al_unwrap_option<T>(opt: Option<T>) -> T {
    opt.unwrap_or_else(|| panic!("AgentLang: unwrap on None"))
}

/// Unwrap a `Result<T, E>`, panicking with an AgentLang-style message if `Err`.
pub fn al_unwrap_result<T, E: std::fmt::Debug>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|e| panic!("AgentLang: unwrap on Err: {:?}", e))
}

/// Unwrap an `Option<T>` or return a default value.
pub fn al_unwrap_or_option<T>(opt: Option<T>, default: T) -> T {
    opt.unwrap_or(default)
}

/// Unwrap a `Result<T, E>` or return a default value.
pub fn al_unwrap_or_result<T, E>(result: Result<T, E>, default: T) -> T {
    result.unwrap_or(default)
}

/// Check if an `Option<T>` is `Some`.
pub fn al_is_some<T>(opt: &Option<T>) -> bool {
    opt.is_some()
}

/// Check if an `Option<T>` is `None`.
pub fn al_is_none<T>(opt: &Option<T>) -> bool {
    opt.is_none()
}

/// Check if a `Result<T, E>` is `Ok`.
pub fn al_is_ok<T, E>(result: &Result<T, E>) -> bool {
    result.is_ok()
}

/// Check if a `Result<T, E>` is `Err`.
pub fn al_is_err<T, E>(result: &Result<T, E>) -> bool {
    result.is_err()
}
