//! Dataset-id collections used by `check`: fixed-capacity (stack) and, under
//! `std`, heap-backed.

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

/// The dataset-id set operations `check` needs, abstracted over storage so
/// the same passes run on the fixed-capacity [`IdSet`] (`no_std`) and the
/// unbounded [`HeapIdSet`] (`std`).
pub(crate) trait IdCollector {
    fn empty() -> Self;
    fn contains(&self, id: usize) -> bool;
    /// Returns `false` only if capacity is exceeded.
    fn insert(&mut self, id: usize) -> bool;
    /// Returns `false` only if capacity is exceeded.
    fn copy_from(&mut self, other: &Self) -> bool;
}

impl<const N: usize> IdCollector for IdSet<N> {
    fn empty() -> Self { Self::new() }
    fn contains(&self, id: usize) -> bool { self.contains(id) }
    fn insert(&mut self, id: usize) -> bool { self.insert(id) }
    fn copy_from(&mut self, other: &Self) -> bool { self.copy_from(other) }
}

/// The `(id, type)` table operation the identity pass needs; see [`IdCollector`].
pub(crate) trait TypeCollector {
    fn empty() -> Self;
    fn insert(&mut self, id: usize, ty: &'static str) -> TypeInsert;
}

impl<const N: usize> TypeCollector for IdTypeMap<N> {
    fn empty() -> Self { Self::new() }
    fn insert(&mut self, id: usize, ty: &'static str) -> TypeInsert { self.insert(id, ty) }
}

/// Heap-backed [`IdCollector`]: never runs out of capacity.
#[cfg(feature = "std")]
pub(crate) struct HeapIdSet(std::collections::HashSet<usize>);

#[cfg(feature = "std")]
impl IdCollector for HeapIdSet {
    fn empty() -> Self { Self(std::collections::HashSet::new()) }
    fn contains(&self, id: usize) -> bool { self.0.contains(&id) }
    fn insert(&mut self, id: usize) -> bool {
        self.0.insert(id);
        true
    }
    fn copy_from(&mut self, other: &Self) -> bool {
        self.0.extend(other.0.iter().copied());
        true
    }
}

/// Heap-backed [`TypeCollector`]: never returns [`TypeInsert::Full`].
#[cfg(feature = "std")]
pub(crate) struct HeapIdTypeMap(std::collections::HashMap<usize, &'static str>);

#[cfg(feature = "std")]
impl TypeCollector for HeapIdTypeMap {
    fn empty() -> Self { Self(std::collections::HashMap::new()) }
    fn insert(&mut self, id: usize, ty: &'static str) -> TypeInsert {
        match self.0.entry(id) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(ty);
                TypeInsert::Inserted
            }
            std::collections::hash_map::Entry::Occupied(e) if *e.get() == ty => TypeInsert::Match,
            std::collections::hash_map::Entry::Occupied(e) => TypeInsert::Conflict(e.get()),
        }
    }
}
