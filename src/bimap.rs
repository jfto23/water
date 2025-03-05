use std::{collections::HashMap, hash::Hash, ops::Deref, rc::Rc};
// map is not needed  (XD)

/// A bijective and bi-drectional map
pub struct BiMap<T, U> {
    left: HashMap<Rc<T>, Rc<U>>,
    right: HashMap<Rc<U>, Rc<T>>,
}

impl<T, U> BiMap<T, U>
where
    T: Eq + Hash,
    U: Eq + Hash,
{
    fn new() -> Self {
        BiMap {
            left: HashMap::new(),
            right: HashMap::new(),
        }
    }

    /// Will overwrite t if t is in left or u if u is in right
    fn insert(&mut self, t: T, u: U) {
        self.remove_by_left(&t);
        self.remove_by_right(&u);
        let t = Rc::new(t);
        let u = Rc::new(u);

        self.left.insert(Rc::clone(&t), Rc::clone(&u));
        self.right.insert(u, t);
    }

    fn get_by_left(&self, t: &T) -> Option<&U> {
        self.left.get(t).map(Deref::deref)
    }

    fn get_by_right(&self, u: &U) -> Option<&T> {
        self.right.get(u).map(Deref::deref)
    }

    fn remove_by_left(&mut self, t: &T) -> Option<U> {
        if let Some(u) = self.left.remove(t) {
            let t = self.right.remove(&u);
            assert!(t.is_some());
            return Some(Rc::try_unwrap(u).ok().unwrap());
        };
        return None;
    }

    fn remove_by_right(&mut self, u: &U) -> Option<T> {
        if let Some(t) = self.right.remove(u) {
            let u = self.left.remove(&t);
            assert!(u.is_some());
            return Some(Rc::try_unwrap(t).ok().unwrap());
        };
        return None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut bimap = BiMap::new();
        bimap.insert("a", 1);
        assert_eq!(bimap.get_by_left(&"a"), Some(&1));
        assert_eq!(bimap.get_by_right(&1), Some(&"a"));
    }

    #[test]
    fn test_remove_left() {
        let mut bimap = BiMap::new();
        bimap.insert("a", 1);
        assert_eq!(bimap.remove_by_left(&"a"), Some(1));
        assert_eq!(bimap.get_by_left(&"a"), None);
        assert_eq!(bimap.get_by_right(&1), None);
    }

    #[test]
    fn test_remove_right() {
        let mut bimap = BiMap::new();
        bimap.insert("a", 1);
        assert_eq!(bimap.remove_by_right(&1), Some("a"));
        assert_eq!(bimap.get_by_left(&"a"), None);
        assert_eq!(bimap.get_by_right(&1), None);
    }

    #[test]
    fn test_overwrite() {
        let mut bimap = BiMap::new();
        bimap.insert("a", 1);
        bimap.insert("b", 1);
        assert_eq!(bimap.get_by_left(&"a"), None);
        assert_eq!(bimap.get_by_left(&"b"), Some(&1));
        assert_eq!(bimap.get_by_right(&1), Some(&"b"));
    }
}
