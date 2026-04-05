// DIVERGENCE: Upstream detects eval during HIR construction in `BuildHIR.ts`
// and immediately bails. In our port, this is a post-HIR validation pass that
// scans for `LoadGlobal("eval")` → `CallExpression` patterns. This only catches
// direct `eval()` calls, not `window.eval()` or indirect eval — matching
// upstream's behavior which also only catches direct `eval()` global calls.

use crate::error::{CompilerError, DiagnosticKind, ErrorCollector};
use crate::hir::types::{HIR, IdSet, IdentifierId, InstructionValue};

/// Detect any call to `eval()` and emit a bail-out error.
pub fn validate_no_eval(hir: &HIR, errors: &mut ErrorCollector) {
    // Phase 1: Collect lvalue IDs of LoadGlobal instructions for 'eval'
    let mut eval_ids: IdSet<IdentifierId> = IdSet::new();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::LoadGlobal { binding } = &instr.value
                && binding.name == "eval"
            {
                eval_ids.insert(instr.lvalue.identifier.id);
            }
        }
    }

    if eval_ids.is_empty() {
        return;
    }

    // Phase 2: Find any CallExpression whose callee is an eval load
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::CallExpression { callee, .. } = &instr.value
                && eval_ids.contains(callee.identifier.id)
            {
                errors.push(CompilerError::invalid_js_with_kind(
                    instr.loc,
                    "Compilation Skipped: The 'eval' function is not supported. Eval is an anti-pattern in JavaScript, and the code executed cannot be evaluated by React Compiler.",
                    DiagnosticKind::EvalUnsupported,
                ));
                return; // One error is enough to bail
            }
        }
    }
}
