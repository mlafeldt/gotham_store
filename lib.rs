// Based on https://github.com/denoland/deno_core/blob/0.290.0/core/gotham_state.rs
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Forked from Gotham:
// https://github.com/gotham-rs/gotham/blob/bcbbf8923789e341b7a0e62c59909428ca4e22e2/gotham/src/state/mod.rs
// Copyright 2017 Gotham Project Developers. MIT license.

//! Easily store and retrieve one value of any Rust type.

#![allow(clippy::should_implement_trait)]

use std::any::{type_name, Any, TypeId};
use std::collections::BTreeMap;

#[derive(Default, Debug)]
pub struct GothamStore {
    data: BTreeMap<TypeId, Box<dyn Any>>,
}

impl std::ops::Deref for GothamStore {
    type Target = BTreeMap<TypeId, Box<dyn Any>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl GothamStore {
    /// Puts a value into the `GothamStore`. One value of each type is retained.
    /// Successive calls to `put` will overwrite the existing value of the same
    /// type.
    pub fn put<T: 'static>(&mut self, t: T) {
        let type_id = TypeId::of::<T>();
        self.data.insert(type_id, Box::new(t));
    }

    /// Determines if the current value exists in `GothamStore`.
    pub fn has<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        self.data.contains_key(&type_id)
    }

    /// Tries to borrow a value from the `GothamStore`.
    pub fn try_borrow<T: 'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.data.get(&type_id).and_then(|b| b.downcast_ref())
    }

    /// Borrows a value from the `GothamStore`.
    pub fn borrow<T: 'static>(&self) -> &T {
        self.try_borrow().unwrap_or_else(|| missing::<T>())
    }

    /// Tries to mutably borrow a value from the `GothamStore`.
    pub fn try_borrow_mut<T: 'static>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.data.get_mut(&type_id).and_then(|b| b.downcast_mut())
    }

    /// Mutably borrows a value from the `GothamStore`.
    pub fn borrow_mut<T: 'static>(&mut self) -> &mut T {
        self.try_borrow_mut().unwrap_or_else(|| missing::<T>())
    }

    /// Tries to move a value out of the `GothamStore` and return ownership.
    pub fn try_take<T: 'static>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();
        self.data
            .remove(&type_id)
            .and_then(|b| b.downcast().ok())
            .map(|b| *b)
    }

    /// Moves a value out of the `GothamStore` and returns ownership.
    ///
    /// # Panics
    ///
    /// If a value of type `T` is not present in `GothamStore`.
    pub fn take<T: 'static>(&mut self) -> T {
        self.try_take().unwrap_or_else(|| missing::<T>())
    }
}

fn missing<T: 'static>() -> ! {
    panic!(
        "required type {} is not present in GothamStore container",
        type_name::<T>()
    );
}

#[cfg(test)]
mod tests {
    use super::GothamStore;

    struct MyStruct {
        value: i32,
    }

    struct AnotherStruct {
        value: &'static str,
    }

    type Alias1 = String;
    type Alias2 = String;

    #[test]
    fn put_borrow1() {
        let mut store = GothamStore::default();
        store.put(MyStruct { value: 1 });
        assert_eq!(store.borrow::<MyStruct>().value, 1);
    }

    #[test]
    fn put_borrow2() {
        let mut store = GothamStore::default();
        assert!(!store.has::<AnotherStruct>());
        store.put(AnotherStruct { value: "a string" });
        assert!(store.has::<AnotherStruct>());
        assert!(!store.has::<MyStruct>());
        store.put(MyStruct { value: 100 });
        assert!(store.has::<MyStruct>());
        assert_eq!(store.borrow::<MyStruct>().value, 100);
        assert_eq!(store.borrow::<AnotherStruct>().value, "a string");
    }

    #[test]
    fn try_borrow() {
        let mut store = GothamStore::default();
        store.put(MyStruct { value: 100 });
        assert!(store.try_borrow::<MyStruct>().is_some());
        assert_eq!(store.try_borrow::<MyStruct>().unwrap().value, 100);
        assert!(store.try_borrow::<AnotherStruct>().is_none());
    }

    #[test]
    fn try_borrow_mut() {
        let mut store = GothamStore::default();
        store.put(MyStruct { value: 100 });
        if let Some(a) = store.try_borrow_mut::<MyStruct>() {
            a.value += 10;
        }
        assert_eq!(store.borrow::<MyStruct>().value, 110);
    }

    #[test]
    fn borrow_mut() {
        let mut store = GothamStore::default();
        store.put(MyStruct { value: 100 });
        {
            let a = store.borrow_mut::<MyStruct>();
            a.value += 10;
        }
        assert_eq!(store.borrow::<MyStruct>().value, 110);
        assert!(store.try_borrow_mut::<AnotherStruct>().is_none());
    }

    #[test]
    fn try_take() {
        let mut store = GothamStore::default();
        store.put(MyStruct { value: 100 });
        assert_eq!(store.try_take::<MyStruct>().unwrap().value, 100);
        assert!(store.try_take::<MyStruct>().is_none());
        assert!(store.try_borrow_mut::<MyStruct>().is_none());
        assert!(store.try_borrow::<MyStruct>().is_none());
        assert!(store.try_take::<AnotherStruct>().is_none());
    }

    #[test]
    fn take() {
        let mut store = GothamStore::default();
        store.put(MyStruct { value: 110 });
        assert_eq!(store.take::<MyStruct>().value, 110);
        assert!(store.try_take::<MyStruct>().is_none());
        assert!(store.try_borrow_mut::<MyStruct>().is_none());
        assert!(store.try_borrow::<MyStruct>().is_none());
    }

    #[test]
    fn type_alias() {
        let mut store = GothamStore::default();
        store.put::<Alias1>("alias1".to_string());
        store.put::<Alias2>("alias2".to_string());
        assert_eq!(store.take::<Alias1>(), "alias2");
        assert!(store.try_take::<Alias1>().is_none());
        assert!(store.try_take::<Alias2>().is_none());
    }

    #[test]
    #[should_panic(
        expected = "required type gotham_store::tests::MyStruct is not present in GothamStore container"
    )]
    fn missing() {
        let store = GothamStore::default();
        let _ = store.borrow::<MyStruct>();
    }

    #[test]
    fn deref() {
        let mut store = GothamStore::default();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        store.put(MyStruct { value: 100 });
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
        store.take::<MyStruct>();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }
}
