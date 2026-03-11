# Utility Data Structures

> Shared utilities used across the compiler pipeline.
> Upstream: `src/Utils/DisjointSet.ts`, various inline utilities

---

### Gap 1: DisjointSet

**Upstream:** `src/Utils/DisjointSet.ts`
**Current state:** `utils/disjoint_set.rs` is a stub.
**What's needed:**

Union-Find data structure with path compression and union by rank. Used critically by `InferReactiveScopeVariables` to group identifiers into reactive scopes.

- Generic `DisjointSet<T: Copy + Eq + Hash>` struct
- Operations:
  - `new()` — create empty set
  - `make_set(item: T)` — add a new item as its own set
  - `find(item: T) -> T` — find representative with path compression
  - `union(a: T, b: T)` — merge sets containing a and b (union by rank)
  - `same_set(a: T, b: T) -> bool` — check if same set
  - `sets() -> Vec<Vec<T>>` — enumerate all disjoint sets
- Must use `IdentifierId` as the key type for scope inference
- Performance: O(alpha(n)) amortized per operation with path compression + union by rank

**Depends on:** None

---

### Gap 2: OrderedMap

**Upstream:** Various uses of ordered iteration in the compiler
**Current state:** `utils/ordered_map.rs` is a stub.
**What's needed:**

May just be a thin wrapper around `indexmap::IndexMap` with compiler-specific convenience methods, or may not be needed if `IndexMap` is used directly. Evaluate during implementation.

- If needed: wrapper providing insertion-order iteration with `FxHasher`
- Consider whether `IndexMap<K, V, FxBuildHasher>` is sufficient everywhere

**Depends on:** None
