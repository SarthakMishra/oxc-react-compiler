#![allow(dead_code)]

use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue, Place};
use rustc_hash::FxHashSet;

/// Known hooks that accept a dependency array as their second argument.
const HOOKS_WITH_DEPS: &[&str] = &["useMemo", "useCallback", "useEffect", "useLayoutEffect"];

/// Validate that dependency arrays for memoization/effect hooks are exhaustive.
///
/// Compares the reactive values actually used inside the callback against the
/// declared dependency array. Missing dependencies can cause stale closures;
/// extra dependencies cause unnecessary re-computations.
pub fn validate_exhaustive_dependencies(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, args } = &instr.value {
                let name = match &callee.identifier.name {
                    Some(n) => n.as_str(),
                    None => continue,
                };

                if !HOOKS_WITH_DEPS.contains(&name) {
                    continue;
                }

                // Need exactly 2 args: callback and deps array
                if args.len() != 2 {
                    continue;
                }

                let callback_place = &args[0];
                let deps_place = &args[1];

                // Find the callback function and collect its free variables
                let callback_deps = collect_callback_dependencies(hir, callback_place);

                // Find the deps array and collect its elements
                let declared_deps = collect_declared_deps(hir, deps_place);

                // Report missing dependencies
                for dep_name in &callback_deps {
                    if !declared_deps.contains(dep_name) {
                        errors.push(CompilerError::invalid_react(
                            instr.loc,
                            format!(
                                "React Hook \"{}\" has a missing dependency: \"{}\". \
                                 Either include it in the dependency array or remove \
                                 the dependency array.",
                                name, dep_name
                            ),
                        ));
                    }
                }
            }
        }
    }
}

/// Collect the names of reactive variables used inside a callback function.
fn collect_callback_dependencies(hir: &HIR, callback: &Place) -> FxHashSet<String> {
    let mut deps = FxHashSet::default();
    let callback_id = callback.identifier.id;

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != callback_id {
                continue;
            }

            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Collect all LoadLocal / LoadContext references inside the function body
                for (_, inner_block) in &lowered_func.body.blocks {
                    for inner_instr in &inner_block.instructions {
                        match &inner_instr.value {
                            InstructionValue::LoadLocal { place }
                            | InstructionValue::LoadContext { place } => {
                                if place.reactive {
                                    if let Some(name) = &place.identifier.name {
                                        deps.insert(name.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    deps
}

/// Collect the names of variables declared in a dependency array expression.
fn collect_declared_deps(hir: &HIR, deps_place: &Place) -> FxHashSet<String> {
    let mut declared = FxHashSet::default();
    let deps_id = deps_place.identifier.id;

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != deps_id {
                continue;
            }

            if let InstructionValue::ArrayExpression { elements } = &instr.value {
                for element in elements {
                    if let crate::hir::types::ArrayElement::Expression(place) = element {
                        if let Some(name) = &place.identifier.name {
                            declared.insert(name.clone());
                        }
                    }
                }
            }
        }
    }

    declared
}
