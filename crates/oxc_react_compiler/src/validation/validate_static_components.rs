#![allow(dead_code)]

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::globals::is_component_name;
use crate::hir::types::{HIR, InstructionValue};

/// Validate that components are not defined inline during render.
///
/// Creating component instances inline causes React to unmount/remount
/// the component on every render, losing all state.
pub fn validate_static_components(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::FunctionExpression { name, .. } = &instr.value
                && let Some(name) = name
                    && is_component_name(name) {
                        // This is a component defined inline - could be problematic
                        // unless it's a known pattern like React.memo(() => ...)
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
