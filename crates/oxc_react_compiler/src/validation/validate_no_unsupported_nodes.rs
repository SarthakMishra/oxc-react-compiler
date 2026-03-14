use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, InstructionValue};

/// Set of `UnsupportedNode` names that upstream explicitly rejects as Todo errors.
///
/// These correspond to patterns where upstream's BuildHIR emits a `Todo` diagnostic
/// and bails out. Other `UnsupportedNode` instructions (e.g., `DebuggerStatement`,
/// statement-level type declarations) are benign and should not cause bail-out.
const REJECTED_UNSUPPORTED_NODES: &[&str] = &[
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle YieldExpression expressions
    "YieldExpression",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle get functions in ObjectExpression
    "ObjectExpression_get_syntax",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle set functions in ObjectExpression
    "ObjectExpression_set_syntax",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle MetaProperty expressions other than import.meta
    "MetaProperty_new_target",
    // Upstream: Todo: (BuildHIR::lowerStatement) Handle for-await loops
    "ForAwaitOfStatement",
    // Upstream: Todo: (BuildHIR::lowerExpression) Handle ClassExpression
    "ClassExpression",
];

/// Validate that the HIR contains no `UnsupportedNode` instructions for patterns
/// that upstream explicitly rejects.
///
/// Upstream emits `Todo` errors during BuildHIR for patterns it doesn't support
/// (e.g., getters/setters in object expressions, `new.target`, `for await` loops,
/// `yield` expressions, class expressions). Our HIRBuilder emits `UnsupportedNode`
/// instructions for these cases. This pass converts the known-rejected ones into
/// `CompilerError::todo()` errors, causing the pipeline to bail — matching upstream.
///
/// This pass runs very early (before any optimization or SSA passes) so that
/// unsupported patterns are caught immediately.
pub fn validate_no_unsupported_nodes(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::UnsupportedNode { node } = &instr.value
                && REJECTED_UNSUPPORTED_NODES.contains(&node.as_str())
            {
                errors.push(CompilerError::todo(
                    instr.loc,
                    format!("(BuildHIR) Unsupported node: {node}"),
                ));
            }

            // Also check nested function bodies
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    validate_no_unsupported_nodes(&lowered_func.body, errors);
                }
                _ => {}
            }
        }
    }
}
