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
    pub fn copy_from(&mut self, other: &Self) -> bool {
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

/// Outcome of inserting an `(id, type)` pair into an [`IdTypeMap`].
pub(crate) enum TypeInsert {
    /// The id was not present; the pair was recorded.
    Inserted,
    /// The id was present with the same type — normal reuse of one dataset.
    Match,
    /// The id was present with a *different* type — two datasets share one
    /// address. Carries the type already recorded.
    Conflict(&'static str),
    /// Capacity exceeded.
    Full,
}

/// A fixed-capacity `usize -> &'static str` table, stack-allocated.
///
/// Used to detect pointer aliasing between datasets of different types: a
/// second insert at a known id with a different type name is a [`Conflict`].
///
/// [`Conflict`]: TypeInsert::Conflict
pub(crate) struct IdTypeMap<const N: usize> {
    entries: [(usize, &'static str); N],
    len: usize,
}

impl<const N: usize> IdTypeMap<N> {
    pub const fn new() -> Self {
        Self { entries: [(0, ""); N], len: 0 }
    }

    /// Record `id` as holding a dataset of type `ty`.
    pub fn insert(&mut self, id: usize, ty: &'static str) -> TypeInsert {
        let mut i = 0;
        while i < self.len {
            let (known_id, known_ty) = self.entries[i];
            if known_id == id {
                return if known_ty == ty {
                    TypeInsert::Match
                } else {
                    TypeInsert::Conflict(known_ty)
                };
            }
            i += 1;
        }
        if self.len >= N {
            return TypeInsert::Full;
        }
        self.entries[self.len] = (id, ty);
        self.len += 1;
        TypeInsert::Inserted
    }
}
