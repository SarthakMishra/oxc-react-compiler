//! Collect optional chain dependency semantics.
//!
//! Maps optional chain expressions (`a?.b?.c`) to their safe dependency
//! representations. Without this pass, the dependency system would track
//! `a?.b.c` as requiring `a.b.c` (which would throw if `a.b` is null).
//!
//! This pass determines the "base" of each optional chain and truncates
//! dependency paths at optional access points, ensuring that scope dependencies
//! only include accesses that are guaranteed safe.

use rustc_hash::FxHashMap;

use crate::hir::types::{DependencyPathEntry, HIR, IdentifierId, InstructionValue, Terminal};

/// Information about an optional chain expression's dependency semantics.
#[derive(Debug, Clone)]
pub struct OptionalChainDep {
    /// The root identifier of the optional chain
    pub root_id: IdentifierId,
    /// The full property path including optional markers
    pub full_path: Vec<DependencyPathEntry>,
    /// The safe dependency path (truncated at last optional access)
    pub safe_path: Vec<DependencyPathEntry>,
}

/// Result of collecting optional chain dependencies.
pub struct OptionalChainDependencies {
    /// Map from terminal instruction → optional chain dependency info.
    /// The "terminal" is the final property load in the chain.
    pub chains: FxHashMap<IdentifierId, OptionalChainDep>,
}

/// Collect optional chain dependency semantics from the HIR.
///
/// Identifies optional chain patterns in the HIR (represented as `Optional`
/// terminals with nested property loads) and computes safe dependency paths
/// for each chain.
pub fn collect_optional_chain_dependencies(hir: &HIR) -> OptionalChainDependencies {
    let mut chains: FxHashMap<IdentifierId, OptionalChainDep> = FxHashMap::default();

    // Build identifier → property path map for chaining resolution
    let mut id_to_path: FxHashMap<IdentifierId, (IdentifierId, Vec<DependencyPathEntry>)> =
        FxHashMap::default();

    // First pass: collect all property loads and their chain relationships
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::PropertyLoad { object, property } = &instr.value {
                let (root_id, mut path) =
                    if let Some((root, existing_path)) = id_to_path.get(&object.identifier.id) {
                        (*root, existing_path.clone())
                    } else {
                        (object.identifier.id, Vec::new())
                    };

                path.push(DependencyPathEntry { property: property.clone(), optional: false });

                id_to_path.insert(instr.lvalue.identifier.id, (root_id, path));
            }
        }
    }

    // Second pass: identify optional chain terminals and mark optional accesses
    for (_, block) in &hir.blocks {
        if let Terminal::Optional { test, consequent, .. } = &block.terminal {
            // The test identifier is the base of the optional chain.
            // Property accesses inside the consequent block that chain off `test`
            // are conditional — they should be marked as optional.
            // Resolve the root and base path: either from a prior PropertyLoad chain
            // or directly from the test identifier (if it's a root variable like `a?.b`)
            let (root, base) =
                if let Some((root_id, base_path)) = id_to_path.get(&test.identifier.id) {
                    (*root_id, base_path.clone())
                } else {
                    (test.identifier.id, Vec::new())
                };
            {
                // Find property loads in the consequent block that chain from the test
                if let Some((_, cons_block)) = hir.blocks.iter().find(|(id, _)| id == consequent) {
                    for instr in &cons_block.instructions {
                        if let InstructionValue::PropertyLoad { object, property } = &instr.value {
                            // Check if this load chains from the optional test
                            if object.identifier.id == test.identifier.id
                                || id_to_path
                                    .get(&object.identifier.id)
                                    .is_some_and(|(r, _)| *r == root)
                            {
                                let mut full_path = base.clone();
                                full_path.push(DependencyPathEntry {
                                    property: property.clone(),
                                    optional: true, // this access is conditional on the optional
                                });

                                // Safe path: truncate at the optional boundary
                                let safe_path = crate::reactive_scopes::scope_dependency_utils::truncate_at_optional(&full_path);

                                chains.insert(
                                    instr.lvalue.identifier.id,
                                    OptionalChainDep { root_id: root, full_path, safe_path },
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    OptionalChainDependencies { chains }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::types::{
        BasicBlock, BlockId, BlockKind, Effect, Identifier, InstructionId, MutableRange, Place,
        Terminal, Type,
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
    fn test_optional_chain_marks_safe_path() {
        // Simulates: a?.b
        // Block 0: t10 = a.x (some prop load for test)
        //          terminal: Optional { test: a, consequent: Block(1), fallthrough: Block(2) }
        // Block 1: t11 = a.b (property load inside optional)
        // Block 2: fallthrough
        let hir = HIR {
            entry: BlockId(0),
            blocks: vec![
                (
                    BlockId(0),
                    BasicBlock {
                        kind: BlockKind::Block,
                        id: BlockId(0),
                        instructions: vec![],
                        terminal: Terminal::Optional {
                            test: make_place(1, "a"),
                            consequent: BlockId(1),
                            fallthrough: BlockId(2),
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
                            11,
                            InstructionValue::PropertyLoad {
                                object: make_place(1, "a"),
                                property: "b".to_string(),
                            },
                        )],
                        terminal: Terminal::Goto { block: BlockId(2) },
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

        let result = collect_optional_chain_dependencies(&hir);
        let chain = result.chains.get(&IdentifierId(11)).unwrap();
        assert_eq!(chain.root_id, IdentifierId(1));
        assert_eq!(chain.full_path.len(), 1);
        assert!(chain.full_path[0].optional);
        assert_eq!(chain.full_path[0].property, "b");
        // Safe path includes the optional access (it's the last one)
        assert_eq!(chain.safe_path.len(), 1);
    }
}
