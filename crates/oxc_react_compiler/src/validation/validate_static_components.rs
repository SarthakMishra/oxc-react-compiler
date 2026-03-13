use rustc_hash::FxHashSet;

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::globals::is_component_name;
use crate::hir::types::{HIR, IdentifierId, InstructionValue};

/// Validate that components are not defined inline during render.
///
/// Creating component instances inline causes React to unmount/remount
/// the component on every render, losing all state.
///
/// Enhanced detection:
/// - PascalCase function expressions → likely inline components
/// - Skip functions wrapped in React.memo() (intentional pattern)
/// - Skip functions that capture reactive context (may be intentional HOC)
pub fn validate_static_components(hir: &HIR, errors: &mut ErrorCollector) {
    // Phase 1: Collect identifiers that are wrapped in React.memo/forwardRef
    // These are intentional patterns and should not be flagged.
    let memo_wrapped = collect_memo_wrapped_ids(hir);

    // Phase 2: Check for inline component definitions
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { name, lowered_func, .. } = &instr.value
                && let Some(name) = name
            {
                if !is_component_name(name) {
                    continue;
                }

                // Skip if this function is wrapped in React.memo/forwardRef
                if memo_wrapped.contains(&instr.lvalue.identifier.id) {
                    continue;
                }

                // Skip if the function captures reactive context — it may be
                // an intentional HOC pattern or render prop
                if !lowered_func.context.is_empty() {
                    // Only warn if the captures include reactive values
                    let has_reactive_capture = lowered_func.context.iter().any(|p| p.reactive);
                    if has_reactive_capture {
                        continue; // likely intentional — captures reactive state
                    }
                }

                errors.push(CompilerError::invalid_react_with_kind(
                    instr.loc,
                    format!(
                        "Component \"{name}\" is defined inline during render. \
                             Move it outside the parent component to avoid remounting."
                    ),
                    DiagnosticKind::StaticComponents,
                ));
            }
        }
    }
}

/// Collect identifier IDs that are passed to React.memo() or React.forwardRef().
fn collect_memo_wrapped_ids(hir: &HIR) -> FxHashSet<IdentifierId> {
    let mut wrapped = FxHashSet::default();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                // React.memo(Component) or memo(Component)
                InstructionValue::CallExpression { callee, args } => {
                    if let Some(name) = callee.identifier.name.as_deref()
                        && (name == "memo" || name == "forwardRef")
                        && !args.is_empty()
                    {
                        wrapped.insert(args[0].identifier.id);
                    }
                }
                // React.memo() as a method call
                InstructionValue::MethodCall { property, args, .. } => {
                    if (property == "memo" || property == "forwardRef") && !args.is_empty() {
                        wrapped.insert(args[0].identifier.id);
                    }
                }
                _ => {}
            }
        }
    }

    wrapped
}
