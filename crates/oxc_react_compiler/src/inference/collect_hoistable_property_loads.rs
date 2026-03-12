
//! Collect property loads that can be safely hoisted to scope entry.
//!
//! A property load `a.b` is hoistable if `a` is guaranteed non-null at that
//! point — i.e., if `a.b` is accessed in a block that executes unconditionally.
//! When `a.b` is unconditionally accessed, `a` is guaranteed non-null for the
//! entire scope, and the load can be hoisted to the scope entry.
//!
//! This feeds into `PropagateScopeDependencies` to produce more precise
//! dependency tracking: instead of depending on `a` (overly broad), we can
//! depend on `a.b` (precise) because we know the access is safe.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::hir::compute_unconditional_blocks::UnconditionalBlocks;
use crate::hir::types::{BlockId, HIR, IdentifierId, InstructionValue};

/// A property load that is guaranteed to execute (safe to hoist).
#[derive(Debug, Clone)]
pub struct HoistablePropertyLoad {
    /// The root identifier being accessed
    pub root_id: IdentifierId,
    /// The root identifier name (for diagnostics)
    pub root_name: Option<String>,
    /// The property path that is safely accessible
    pub path: Vec<String>,
    /// The block where this access was found
    pub block: BlockId,
}

/// Result of collecting hoistable property loads.
pub struct HoistableLoads {
    /// Map from root identifier → set of property paths guaranteed accessible.
    /// If `props` → `{["a"], ["a", "b"], ["c"]}`, then `props.a`, `props.a.b`,
    /// and `props.c` are all safe to access without null checks.
    pub loads: FxHashMap<IdentifierId, Vec<Vec<String>>>,
    /// Set of identifiers known to be non-null (they have unconditional property access).
    pub non_null_objects: FxHashSet<IdentifierId>,
}

/// Collect property loads from unconditional blocks that are safe to hoist.
///
/// Scans all unconditional blocks for `PropertyLoad` instructions and records
/// which property paths are guaranteed to execute. This provides non-null
/// guarantees that enable more precise dependency tracking.
pub fn collect_hoistable_property_loads(
    hir: &HIR,
    unconditional: &UnconditionalBlocks,
) -> HoistableLoads {
    let mut loads: FxHashMap<IdentifierId, Vec<Vec<String>>> = FxHashMap::default();
    let mut non_null_objects: FxHashSet<IdentifierId> = FxHashSet::default();

    // Build a map from identifier ID to accumulated property path.
    // When we see `t1 = a.b` and then `t2 = t1.c`, we know `a.b.c` is accessed.
    let mut id_to_path: FxHashMap<IdentifierId, (IdentifierId, Vec<String>)> = FxHashMap::default();

    for (bid, block) in &hir.blocks {
        if !unconditional.unconditional.contains(bid) {
            continue; // only process unconditional blocks
        }

        for instr in &block.instructions {
            if let InstructionValue::PropertyLoad { object, property } = &instr.value {
                let (root_id, mut path) =
                    if let Some((root, existing_path)) = id_to_path.get(&object.identifier.id) {
                        (*root, existing_path.clone())
                    } else {
                        (object.identifier.id, Vec::new())
                    };

                path.push(property.clone());

                // Record this load result's ID for chaining
                id_to_path.insert(instr.lvalue.identifier.id, (root_id, path.clone()));

                // Record the hoistable load
                non_null_objects.insert(root_id);
                loads.entry(root_id).or_default().push(path);
            }
        }
    }

    HoistableLoads { loads, non_null_objects }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::types::{
        BasicBlock, BlockKind, Effect, Identifier, InstructionId, MutableRange, Place, Terminal,
        Type,
    };
    use oxc_span::Span;

    fn make_place(id: u32, name: &str) -> Place {
        Place {
            identifier: Identifier {
                id: IdentifierId(id),
                declaration_id: None,
                name: Some(name.to_string()),
                mutable_range: MutableRange { start: InstructionId(0), end: InstructionId(0) },
                scope: None,
                type_: Type::default(),
                loc: Span::default(),
            },
            effect: Effect::Unknown,
            reactive: false,
            loc: Span::default(),
        }
    }

    fn make_instr(
        id: u32,
        lvalue_id: u32,
        value: InstructionValue,
    ) -> crate::hir::types::Instruction {
        crate::hir::types::Instruction {
            id: InstructionId(id),
            lvalue: make_place(lvalue_id, &format!("t{lvalue_id}")),
            value,
            loc: Span::default(),
            effects: None,
        }
    }

    #[test]
    fn test_collects_unconditional_property_loads() {
        let hir = HIR {
            entry: BlockId(0),
            blocks: vec![(
                BlockId(0),
                BasicBlock {
                    kind: BlockKind::Block,
                    id: BlockId(0),
                    instructions: vec![make_instr(
                        1,
                        10,
                        InstructionValue::PropertyLoad {
                            object: make_place(1, "props"),
                            property: "name".to_string(),
                        },
                    )],
                    terminal: Terminal::Return { value: make_place(0, "undefined") },
                    preds: Vec::new(),
                    phis: Vec::new(),
                },
            )],
        };

        let mut unconditional_set = FxHashSet::default();
        unconditional_set.insert(BlockId(0));
        let unconditional = UnconditionalBlocks {
            unconditional: unconditional_set,
            postdominators: FxHashMap::default(),
        };

        let result = collect_hoistable_property_loads(&hir, &unconditional);
        assert!(result.non_null_objects.contains(&IdentifierId(1)));
        let paths = result.loads.get(&IdentifierId(1)).unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], vec!["name"]);
    }

    #[test]
    fn test_chains_property_loads() {
        // t10 = props.a; t11 = t10.b  →  should record props → [["a"], ["a", "b"]]
        let hir = HIR {
            entry: BlockId(0),
            blocks: vec![(
                BlockId(0),
                BasicBlock {
                    kind: BlockKind::Block,
                    id: BlockId(0),
                    instructions: vec![
                        make_instr(
                            1,
                            10,
                            InstructionValue::PropertyLoad {
                                object: make_place(1, "props"),
                                property: "a".to_string(),
                            },
                        ),
                        make_instr(
                            2,
                            11,
                            InstructionValue::PropertyLoad {
                                object: make_place(10, "t10"),
                                property: "b".to_string(),
                            },
                        ),
                    ],
                    terminal: Terminal::Return { value: make_place(0, "undefined") },
                    preds: Vec::new(),
                    phis: Vec::new(),
                },
            )],
        };

        let mut unconditional_set = FxHashSet::default();
        unconditional_set.insert(BlockId(0));
        let unconditional = UnconditionalBlocks {
            unconditional: unconditional_set,
            postdominators: FxHashMap::default(),
        };

        let result = collect_hoistable_property_loads(&hir, &unconditional);
        let paths = result.loads.get(&IdentifierId(1)).unwrap();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&vec!["a".to_string()]));
        assert!(paths.contains(&vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_skips_conditional_blocks() {
        let hir = HIR {
            entry: BlockId(0),
            blocks: vec![
                (
                    BlockId(0),
                    BasicBlock {
                        kind: BlockKind::Block,
                        id: BlockId(0),
                        instructions: vec![],
                        terminal: Terminal::Branch {
                            test: make_place(0, "cond"),
                            consequent: BlockId(1),
                            alternate: BlockId(2),
                        },
                        preds: Vec::new(),
                        phis: Vec::new(),
                    },
                ),
                (
                    BlockId(1),
                    BasicBlock {
                        kind: BlockKind::Block,
                        id: BlockId(1),
                        instructions: vec![make_instr(
                            1,
                            10,
                            InstructionValue::PropertyLoad {
                                object: make_place(1, "props"),
                                property: "name".to_string(),
                            },
                        )],
                        terminal: Terminal::Return { value: make_place(0, "undefined") },
                        preds: Vec::new(),
                        phis: Vec::new(),
                    },
                ),
                (
                    BlockId(2),
                    BasicBlock {
                        kind: BlockKind::Block,
                        id: BlockId(2),
                        instructions: vec![],
                        terminal: Terminal::Return { value: make_place(0, "undefined") },
                        preds: Vec::new(),
                        phis: Vec::new(),
                    },
                ),
            ],
        };

        // Only Block(0) is unconditional, Block(1) and Block(2) are conditional
        let mut unconditional_set = FxHashSet::default();
        unconditional_set.insert(BlockId(0));
        let unconditional = UnconditionalBlocks {
            unconditional: unconditional_set,
            postdominators: FxHashMap::default(),
        };

        let result = collect_hoistable_property_loads(&hir, &unconditional);
        assert!(result.loads.is_empty()); // No loads in unconditional blocks
        assert!(result.non_null_objects.is_empty());
    }
}
