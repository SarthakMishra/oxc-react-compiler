use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{HIR, InstructionKind, InstructionValue};

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
/// that upstream explicitly rejects, and detect other unsupported syntax patterns.
///
/// Upstream emits `Todo` errors during BuildHIR for patterns it doesn't support
/// (e.g., getters/setters in object expressions, `new.target`, `for await` loops,
/// `yield` expressions, class expressions). Our HIRBuilder emits `UnsupportedNode`
/// instructions for these cases. This pass converts the known-rejected ones into
/// `CompilerError::todo()` errors, causing the pipeline to bail — matching upstream.
///
/// Additionally detects:
/// - `var` declarations: upstream bails with "Todo: Handle var kinds in VariableDeclaration"
///   because `var` hoisting semantics are not correctly modeled in the HIR.
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

            // Upstream: Todo: (BuildHIR::lowerStatement) Handle var kinds in VariableDeclaration
            // `var` declarations have function-level hoisting semantics that our HIR
            // does not model correctly. Upstream bails on these.
            check_var_declaration(instr, errors);

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

/// Check if an instruction declares a `var` variable.
/// Upstream does not support `var` declarations because their function-level
/// hoisting semantics are incompatible with the block-scoped HIR model.
fn check_var_declaration(instr: &crate::hir::types::Instruction, errors: &mut ErrorCollector) {
    match &instr.value {
        InstructionValue::DeclareLocal { type_: InstructionKind::Var, .. }
        | InstructionValue::StoreLocal { type_: Some(InstructionKind::Var), .. } => {
            errors.push(CompilerError::todo(
                instr.loc,
                "(BuildHIR::lowerStatement) Handle var kinds in VariableDeclaration".to_string(),
            ));
        }
        _ => {}
    }
}
