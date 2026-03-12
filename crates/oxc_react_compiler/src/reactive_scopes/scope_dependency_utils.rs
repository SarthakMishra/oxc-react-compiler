#![allow(dead_code)]

//! Shared utilities for manipulating reactive scope dependencies.
//!
//! These utilities are used by:
//! - `propagate_dependencies.rs` for initial dependency collection
//! - `derive_minimal_dependencies.rs` for dependency tree minimization
//! - `collect_optional_chain_dependencies.rs` for optional chain semantics
//! - `collect_hoistable_property_loads.rs` for property load hoisting

use std::cmp::Ordering;

use crate::hir::types::{DependencyPathEntry, IdentifierId, ReactiveScopeDependency};

/// Compare two dependency paths to determine if one is a prefix of the other.
///
/// Returns:
/// - `Some(Ordering::Equal)` if paths are identical
/// - `Some(Ordering::Less)` if `a` is a strict prefix of `b` (a.x vs a.x.y)
/// - `Some(Ordering::Greater)` if `b` is a strict prefix of `a`
/// - `None` if paths diverge (a.x vs a.y)
pub fn compare_paths(a: &[DependencyPathEntry], b: &[DependencyPathEntry]) -> Option<Ordering> {
    let min_len = a.len().min(b.len());
    for i in 0..min_len {
        if a[i].property != b[i].property {
            return None; // paths diverge
        }
    }
    Some(a.len().cmp(&b.len()))
}

/// Check if dependency `a` subsumes `b`, meaning tracking `a` makes `b` redundant.
///
/// A dependency `a` subsumes `b` when:
/// - They refer to the same root identifier
/// - `a`'s path is a prefix of (or equal to) `b`'s path
/// - `a` doesn't have optional access where `b` has non-optional
///
/// For example, `props` subsumes `props.a`, and `props.a` subsumes `props.a.b`.
pub fn dependency_subsumes(a: &ReactiveScopeDependency, b: &ReactiveScopeDependency) -> bool {
    if a.identifier.id != b.identifier.id {
        return false;
    }
    match compare_paths(&a.path, &b.path) {
        Some(Ordering::Less | Ordering::Equal) => {
            // a's path is a prefix of b's path — a subsumes b,
            // but only if a doesn't introduce optional access that b doesn't have
            for (i, entry) in a.path.iter().enumerate() {
                if entry.optional && !b.path[i].optional {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

/// Merge two sets of dependencies, removing redundant entries.
///
/// If `deps_a` contains `props` and `deps_b` contains `props.name`,
/// the result will only contain `props` (the more general dependency).
pub fn merge_dependencies(
    deps_a: &[ReactiveScopeDependency],
    deps_b: &[ReactiveScopeDependency],
) -> Vec<ReactiveScopeDependency> {
    let mut merged: Vec<ReactiveScopeDependency> = Vec::new();
    for dep in deps_a.iter().chain(deps_b.iter()) {
        add_dependency(&mut merged, dep.clone());
    }
    merged
}

/// Add a dependency to a list, deduplicating and removing subsumed entries.
///
/// If the new dependency subsumes existing entries, they are removed.
/// If an existing entry subsumes the new dependency, it is not added.
pub fn add_dependency(deps: &mut Vec<ReactiveScopeDependency>, new_dep: ReactiveScopeDependency) {
    // Check if any existing dep subsumes the new one
    for existing in deps.iter() {
        if dependency_subsumes(existing, &new_dep) {
            return; // already covered
        }
    }

    // Remove any existing deps subsumed by the new one
    deps.retain(|existing| !dependency_subsumes(&new_dep, existing));
    deps.push(new_dep);
}

/// Normalize a dependency path by collapsing optional chains.
///
/// For a path like `a?.b.c`, this determines the "safe" dependency:
/// if `a` might be nullish, we can only safely depend on `a` (not `a.b.c`).
///
/// Returns the truncated path up to and including the last optional access.
pub fn truncate_at_optional(path: &[DependencyPathEntry]) -> Vec<DependencyPathEntry> {
    // Find the last optional entry - everything after it is conditional
    let mut last_optional_idx = None;
    for (i, entry) in path.iter().enumerate() {
        if entry.optional {
            last_optional_idx = Some(i);
        }
    }

    match last_optional_idx {
        Some(idx) => path[..=idx].to_vec(),
        None => path.to_vec(), // no optional access, path is safe as-is
    }
}

/// Group dependencies by their root identifier.
pub fn group_by_root(
    deps: &[ReactiveScopeDependency],
) -> rustc_hash::FxHashMap<IdentifierId, Vec<&ReactiveScopeDependency>> {
    let mut groups: rustc_hash::FxHashMap<IdentifierId, Vec<&ReactiveScopeDependency>> =
        rustc_hash::FxHashMap::default();
    for dep in deps {
        groups.entry(dep.identifier.id).or_default().push(dep);
    }
    groups
}

/// Check if two dependencies refer to the same root and path.
pub fn dependencies_equal(a: &ReactiveScopeDependency, b: &ReactiveScopeDependency) -> bool {
    a.identifier.id == b.identifier.id && paths_equal(&a.path, &b.path)
}

/// Check if two dependency paths are structurally equal.
pub fn paths_equal(a: &[DependencyPathEntry], b: &[DependencyPathEntry]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b.iter()).all(|(x, y)| x.property == y.property && x.optional == y.optional)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::types::{Identifier, InstructionId, MutableRange};
    use oxc_span::Span;

    fn make_dep(id: u32, path: &[(&str, bool)]) -> ReactiveScopeDependency {
        ReactiveScopeDependency {
            identifier: Identifier {
                id: IdentifierId(id),
                declaration_id: None,
                name: Some(format!("v{id}")),
                mutable_range: MutableRange { start: InstructionId(0), end: InstructionId(0) },
                scope: None,
                type_: crate::hir::types::Type::default(),
                loc: Span::default(),
            },
            reactive: true,
            path: path
                .iter()
                .map(|(p, o)| DependencyPathEntry { property: p.to_string(), optional: *o })
                .collect(),
        }
    }

    #[test]
    fn test_compare_paths_equal() {
        let a = vec![DependencyPathEntry { property: "x".into(), optional: false }];
        let b = vec![DependencyPathEntry { property: "x".into(), optional: false }];
        assert_eq!(compare_paths(&a, &b), Some(Ordering::Equal));
    }

    #[test]
    fn test_compare_paths_prefix() {
        let a = vec![DependencyPathEntry { property: "x".into(), optional: false }];
        let b = vec![
            DependencyPathEntry { property: "x".into(), optional: false },
            DependencyPathEntry { property: "y".into(), optional: false },
        ];
        assert_eq!(compare_paths(&a, &b), Some(Ordering::Less));
    }

    #[test]
    fn test_compare_paths_diverge() {
        let a = vec![DependencyPathEntry { property: "x".into(), optional: false }];
        let b = vec![DependencyPathEntry { property: "y".into(), optional: false }];
        assert_eq!(compare_paths(&a, &b), None);
    }

    #[test]
    fn test_subsumes_same_root_prefix() {
        let a = make_dep(1, &[("a", false)]);
        let b = make_dep(1, &[("a", false), ("b", false)]);
        assert!(dependency_subsumes(&a, &b));
        assert!(!dependency_subsumes(&b, &a));
    }

    #[test]
    fn test_subsumes_different_root() {
        let a = make_dep(1, &[("a", false)]);
        let b = make_dep(2, &[("a", false)]);
        assert!(!dependency_subsumes(&a, &b));
    }

    #[test]
    fn test_subsumes_root_only() {
        let a = make_dep(1, &[]);
        let b = make_dep(1, &[("a", false)]);
        assert!(dependency_subsumes(&a, &b));
    }

    #[test]
    fn test_merge_removes_subsumed() {
        let a = vec![make_dep(1, &[])]; // props
        let b = vec![make_dep(1, &[("name", false)])]; // props.name
        let merged = merge_dependencies(&a, &b);
        assert_eq!(merged.len(), 1);
        assert!(merged[0].path.is_empty()); // kept the root-level dep
    }

    #[test]
    fn test_truncate_at_optional() {
        let path = vec![
            DependencyPathEntry { property: "a".into(), optional: false },
            DependencyPathEntry { property: "b".into(), optional: true },
            DependencyPathEntry { property: "c".into(), optional: false },
        ];
        let truncated = truncate_at_optional(&path);
        assert_eq!(truncated.len(), 2);
        assert_eq!(truncated[1].property, "b");
    }

    #[test]
    fn test_truncate_no_optional() {
        let path = vec![
            DependencyPathEntry { property: "a".into(), optional: false },
            DependencyPathEntry { property: "b".into(), optional: false },
        ];
        let truncated = truncate_at_optional(&path);
        assert_eq!(truncated.len(), 2);
    }

    #[test]
    fn test_add_dependency_dedup() {
        let mut deps = vec![make_dep(1, &[("a", false), ("b", false)])]; // props.a.b
        add_dependency(&mut deps, make_dep(1, &[("a", false)])); // props.a (subsumes props.a.b)
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].path.len(), 1); // kept the shorter path
    }
}
