//! Derive minimal set of dependencies for each reactive scope.
//!
//! If a scope depends on both `props.a` and `props.a.b`, we only need to track
//! `props.a` since invalidating `props.a` implies `props.a.b` also changed.
//!
//! This pass operates as a trie-based minimization: for each root identifier,
//! we build a tree of property paths and prune children whose parent is already
//! a dependency.

use crate::hir::types::HIR;
use crate::reactive_scopes::scope_dependency_utils::add_dependency;

/// Minimize the dependency set for each reactive scope in the HIR.
///
/// For each scope, reduces the dependency list to the minimal set where
/// no dependency is subsumed by another.
pub fn derive_minimal_dependencies_hir(hir: &mut HIR) {
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            if let Some(ref mut scope) = instr.lvalue.identifier.scope {
                if scope.dependencies.len() <= 1 {
                    continue; // nothing to minimize
                }
                let original = std::mem::take(&mut scope.dependencies);
                let mut minimal = Vec::with_capacity(original.len());
                for dep in original {
                    add_dependency(&mut minimal, dep);
                }
                scope.dependencies = minimal;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::types::{
        BasicBlock, BlockId, BlockKind, DependencyPathEntry, Effect, Identifier, IdentifierId,
        Instruction, InstructionId, InstructionValue, MutableRange, Place, ReactiveScope,
        ReactiveScopeDependency, ScopeId, Terminal, Type,
    };
    use oxc_span::Span;

    fn make_dep(id: u32, path: &[&str]) -> ReactiveScopeDependency {
        ReactiveScopeDependency {
            identifier: Identifier {
                id: IdentifierId(id),
                declaration_id: None,
                name: Some(format!("v{id}")),
                mutable_range: MutableRange { start: InstructionId(0), end: InstructionId(0) },
                scope: None,
                type_: Type::default(),
                loc: Span::default(),
            },
            reactive: true,
            path: path
                .iter()
                .map(|p| DependencyPathEntry { property: p.to_string(), optional: false })
                .collect(),
        }
    }

    fn make_instruction_with_scope(deps: Vec<ReactiveScopeDependency>) -> Instruction {
        Instruction {
            id: InstructionId(0),
            lvalue: Place {
                identifier: Identifier {
                    id: IdentifierId(99),
                    declaration_id: None,
                    name: None,
                    mutable_range: MutableRange { start: InstructionId(0), end: InstructionId(0) },
                    scope: Some(Box::new(ReactiveScope {
                        id: ScopeId(0),
                        range: MutableRange { start: InstructionId(0), end: InstructionId(10) },
                        dependencies: deps,
                        declarations: Vec::new(),
                        reassignments: Vec::new(),
                        early_return_value: None,
                        merged: Vec::new(),
                        loc: Span::default(),
                    })),
                    type_: Type::default(),
                    loc: Span::default(),
                },
                effect: Effect::Unknown,
                reactive: false,
                loc: Span::default(),
            },
            value: InstructionValue::Primitive { value: crate::hir::types::Primitive::Undefined },
            loc: Span::default(),
            effects: None,
        }
    }

    #[test]
    fn test_minimize_removes_subsumed_dep() {
        let deps = vec![
            make_dep(1, &["a"]),      // props.a
            make_dep(1, &["a", "b"]), // props.a.b (subsumed by props.a)
        ];
        let mut hir = HIR {
            entry: BlockId(0),
            blocks: vec![(
                BlockId(0),
                BasicBlock {
                    kind: BlockKind::Block,
                    id: BlockId(0),
                    instructions: vec![make_instruction_with_scope(deps)],
                    terminal: Terminal::Return {
                        value: Place {
                            identifier: Identifier {
                                id: IdentifierId(0),
                                declaration_id: None,
                                name: None,
                                mutable_range: MutableRange {
                                    start: InstructionId(0),
                                    end: InstructionId(0),
                                },
                                scope: None,
                                type_: Type::default(),
                                loc: Span::default(),
                            },
                            effect: Effect::Unknown,
                            reactive: false,
                            loc: Span::default(),
                        },
                    },
                    preds: Vec::new(),
                    phis: Vec::new(),
                },
            )],
        };

        derive_minimal_dependencies_hir(&mut hir);

        let scope = hir.blocks[0].1.instructions[0].lvalue.identifier.scope.as_ref().unwrap();
        assert_eq!(scope.dependencies.len(), 1);
        assert_eq!(scope.dependencies[0].path.len(), 1);
        assert_eq!(scope.dependencies[0].path[0].property, "a");
    }

    #[test]
    fn test_keeps_divergent_paths() {
        let deps = vec![
            make_dep(1, &["a"]), // props.a
            make_dep(1, &["b"]), // props.b (different path, not subsumed)
        ];
        let mut hir = HIR {
            entry: BlockId(0),
            blocks: vec![(
                BlockId(0),
                BasicBlock {
                    kind: BlockKind::Block,
                    id: BlockId(0),
                    instructions: vec![make_instruction_with_scope(deps)],
                    terminal: Terminal::Return {
                        value: Place {
                            identifier: Identifier {
                                id: IdentifierId(0),
                                declaration_id: None,
                                name: None,
                                mutable_range: MutableRange {
                                    start: InstructionId(0),
                                    end: InstructionId(0),
                                },
                                scope: None,
                                type_: Type::default(),
                                loc: Span::default(),
                            },
                            effect: Effect::Unknown,
                            reactive: false,
                            loc: Span::default(),
                        },
                    },
                    preds: Vec::new(),
                    phis: Vec::new(),
                },
            )],
        };

        derive_minimal_dependencies_hir(&mut hir);

        let scope = hir.blocks[0].1.instructions[0].lvalue.identifier.scope.as_ref().unwrap();
        assert_eq!(scope.dependencies.len(), 2);
    }
}
