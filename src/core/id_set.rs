//! Fixed-capacity set of `usize` values, stack-allocated.

/// A simple set backed by a flat array. No allocator needed.
///
/// `N` is the maximum number of distinct values the set can hold.
pub(crate) struct IdSet<const N: usize> {
    ids: [usize; N],
    len: usize,
}

impl<const N: usize> IdSet<N> {
    pub const fn new() -> Self {
        Self { ids: [0; N], len: 0 }
    }

    pub fn contains(&self, id: usize) -> bool {
        let mut i = 0;
        while i < self.len {
            if self.ids[i] == id {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Insert a value. Returns `true` on success, `false` if capacity exceeded.
    /// If the value is already present, returns `true` without duplicating.
    pub fn insert(&mut self, id: usize) -> bool {
        if self.contains(id) {
            return true;
        }
        if self.len >= N {
            return false;
        }
        self.ids[self.len] = id;
        self.len += 1;
        true
    }

    /// Copy all entries from `other` into `self`. Returns `false` if capacity exceeded.
    pub fn copy_from(&mut self, other: &IdSet<N>) -> bool {
        let mut i = 0;
        while i < other.len {
            if !self.insert(other.ids[i]) {
                return false;
            }
            i += 1;
        }
        true
    }
}
