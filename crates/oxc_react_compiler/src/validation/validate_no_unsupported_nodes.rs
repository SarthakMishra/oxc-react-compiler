use crate::error::{CompilerError, ErrorCollector};
use crate::hir::types::{
    BlockId, HIR, HIRFunction, IdentifierId, Instruction, InstructionKind, InstructionValue, Param,
    Terminal,
};
use rustc_hash::FxHashSet;

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
    // Upstream: Todo: (BuildHIR::lowerStatement) Handle TryStatement without a catch clause
    "TryStatement_without_catch",
    // Note: ThrowStatement in try/catch is detected via CFG walk in check_value_blocks_in_try,
    // not via UnsupportedNode markers, so it doesn't appear in this list.
    // Upstream: Invariant: (BuildHIR::lowerAssignment) Could not find binding for declaration.
    // Destructured catch clause parameters (e.g. `catch ({status})`) are not supported.
    "CatchClause_destructured_param",
    // Upstream: Todo: Support spread syntax for hook arguments
    // Hook calls with spread arguments (e.g. `useHook(...items)`) are not supported.
    "HookCall_spread_argument",
    // Upstream: Todo: Expression type `ArrowFunctionExpression` cannot be safely reordered
    // Default parameter values that are arrow/function expressions cannot be reordered.
    "DefaultParam_nonreorderable_expression",
    // Upstream: Invariant: Const declaration cannot be referenced as an expression
    // Nested destructuring in assignment expressions (e.g. `([[x]] = makeObject())`)
    // causes an invariant failure in upstream's codegen.
    "NestedDestructuringAssignment",
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
/// - Value blocks in try/catch: upstream bails when conditional/logical/optional/loop
///   expressions appear inside try blocks.
/// - Local variables named `fbt`: upstream bails on these.
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

            // Upstream: Todo: Support local variables named `fbt`
            // fbt is a special module for Facebook internationalization. When fbt is
            // used as a local variable name, upstream bails.
            check_fbt_local(instr, errors);

            // Also check nested function bodies
            match &instr.value {
                InstructionValue::FunctionExpression { lowered_func, .. }
                | InstructionValue::ObjectMethod { lowered_func, .. } => {
                    // Upstream: Todo: Support local variables named `fbt`
                    // Check for `fbt` as a function parameter name.
                    check_fbt_in_function_params(lowered_func, errors);
                    // Upstream: Todo: Handle UpdateExpression to variables captured within lambdas
                    check_update_context_identifiers(&lowered_func.body, instr.loc, errors);
                    validate_no_unsupported_nodes(&lowered_func.body, errors);
                }
                _ => {}
            }
        }
    }

    // DIVERGENCE: Upstream (pre-compilationMode:"all") bailed on value blocks inside
    // try/catch (for-loops, ternaries, logical/optional chaining) AND throw statements.
    // With compilationMode:"all", upstream now compiles value blocks successfully, but
    // throw-in-try is still an error. We only check for throw statements now.
    check_throw_in_try(hir, errors);

    // Check for function declarations in unreachable code (after return/throw).
    // Upstream: Todo: Support functions with unreachable code that may contain hoisted declarations
    check_hoisted_function_in_unreachable_code(hir, errors);

    // DIVERGENCE: Upstream codegen fails with "Invariant: [Codegen] Internal error:
    // MethodCall::property must be an unpromoted + unmemoized MemberExpression" when
    // a MethodCall result is used as an argument to another MethodCall. We detect this
    // pattern early and bail to match upstream's behavior.
    check_nested_method_call_as_argument(hir, errors);

    // Upstream: Todo: [hoisting] EnterSSA: Expected identifier to be defined before being used
    // Detect self-referencing declarations like `const x = identity(x)` where x is
    // loaded after DeclareLocal but before the corresponding StoreLocal initialization.
    // This is a TDZ (Temporal Dead Zone) violation in JavaScript semantics.
    check_self_referencing_declarations(hir, errors);

    // Upstream: Todo: Support duplicate fbt tags
    // When an <fbt> element contains multiple <fbt:enum>, <fbt:plural>, or <fbt:pronoun>
    // children (lowered to fbt._enum(), fbt._plural(), fbt._pronoun() calls), upstream
    // bails because the fbt Babel plugin has deduplication issues with synthesized nodes.
    check_fbt_duplicate_tags(hir, errors);
}

/// Check if an instruction declares a `var` variable.
/// Upstream does not support `var` declarations because their function-level
/// hoisting semantics are incompatible with the block-scoped HIR model.
fn check_var_declaration(instr: &Instruction, errors: &mut ErrorCollector) {
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

/// Check if an instruction declares a local variable named `fbt`.
/// Upstream: Todo: Support local variables named `fbt`
fn check_fbt_local(instr: &Instruction, errors: &mut ErrorCollector) {
    if let InstructionValue::DeclareLocal { lvalue, .. }
    | InstructionValue::StoreLocal { lvalue, .. } = &instr.value
        && let Some(name) = &lvalue.identifier.name
        && name == "fbt"
    {
        errors.push(CompilerError::todo(
            instr.loc,
            "Support local variables named `fbt`".to_string(),
        ));
    }
}

/// Check if a `HIRFunction` has a parameter named `fbt`.
/// Upstream: Todo: Support local variables named `fbt`
/// When `fbt` appears as a function parameter name (e.g., `fbt => fbt._("...")`),
/// it creates a local variable that conflicts with the fbt plugin transformation.
fn check_fbt_in_function_params(func: &HIRFunction, errors: &mut ErrorCollector) {
    for param in &func.params {
        let place = match param {
            Param::Identifier(p) | Param::Spread(p) => p,
        };
        if place.identifier.name.as_deref() == Some("fbt") {
            errors.push(CompilerError::todo(
                func.loc,
                "Support local variables named `fbt`".to_string(),
            ));
            return;
        }
    }
}

/// Check for `UpdateExpression` (`++`/`--`) applied to variables captured from
/// outer scope (context variables) within nested function expressions.
/// Upstream: Todo: (BuildHIR::lowerExpression) Handle UpdateExpression to variables
/// captured within lambdas.
///
/// Detects the pattern: `let x = 0; const fn = () => { x++; };`
/// The `x++` modifies a context variable which upstream cannot handle.
fn check_update_context_identifiers(
    func_hir: &HIR,
    func_loc: oxc_span::Span,
    errors: &mut ErrorCollector,
) {
    // Collect all identifier IDs that appear as LoadContext/StoreContext targets.
    // These are variables captured from the outer scope.
    let mut context_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &func_hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadContext { place }
                | InstructionValue::StoreContext { lvalue: place, .. } => {
                    context_ids.insert(place.identifier.id);
                }
                _ => {}
            }
        }
    }
    if context_ids.is_empty() {
        return;
    }
    // Check for PrefixUpdate/PostfixUpdate on context variables
    for (_, block) in &func_hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::PrefixUpdate { lvalue, .. }
                | InstructionValue::PostfixUpdate { lvalue, .. } => {
                    if context_ids.contains(&lvalue.identifier.id) {
                        errors.push(CompilerError::todo(
                            func_loc,
                            "(BuildHIR::lowerExpression) Handle UpdateExpression to variables captured within lambdas".to_string(),
                        ));
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

/// Check for function declarations in unreachable code (after return/throw).
/// Upstream: Todo: Support functions with unreachable code that may contain
/// hoisted declarations.
///
/// When a function declaration appears after a `return` or `throw` statement,
/// it is placed in a dead block (no predecessors) in our HIR. Upstream bails
/// because hoisted function declarations have complex semantics in unreachable
/// code — they are still JS-hoisted to the top of the containing scope.
fn check_hoisted_function_in_unreachable_code(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        // Skip the entry block — it has no preds but is reachable.
        if block.id == hir.entry {
            continue;
        }
        // Dead block: no predecessors means unreachable
        if block.preds.is_empty() {
            for instr in &block.instructions {
                if matches!(
                    &instr.value,
                    InstructionValue::DeclareLocal { type_: InstructionKind::HoistedFunction, .. }
                ) {
                    errors.push(CompilerError::todo(
                        instr.loc,
                        "Support functions with unreachable code that may contain hoisted declarations".to_string(),
                    ));
                    return; // One error per HIR is enough
                }
            }
        }
    }
}

/// Check for "value blocks" (conditional, logical, optional chaining, for loops)
/// inside try/catch statements.
///
/// Upstream: Todo: Support value blocks (conditional, logical, optional chaining, etc)
/// within a try/catch statement.
///
/// Upstream's BuildHIR detects when lowering an expression inside a try block
/// would create "value blocks" — intermediate blocks for evaluating conditional
/// expressions, logical expressions, optional chains, or loop constructs.
/// These value blocks break the try/catch control flow model because exceptions
/// thrown inside them would not be properly caught.
///
/// We detect this by walking the HIR CFG: for each Try terminal, collect all
/// blocks reachable from the try body (not the handler), and check if any of
/// those blocks contain terminals that create value blocks (conditional, logical,
/// optional, for/for-in/for-of).
/// Check for ThrowStatement inside try bodies. Upstream still bails on this
/// even with compilationMode:"all". Value blocks (for, ternary, logical, optional)
/// are now handled correctly and no longer cause bail.
fn check_throw_in_try(hir: &HIR, errors: &mut ErrorCollector) {
    let block_map: rustc_hash::FxHashMap<BlockId, &crate::hir::types::BasicBlock> =
        hir.blocks.iter().map(|(id, block)| (*id, block)).collect();

    for (_, block) in &hir.blocks {
        if let Terminal::Try { block: try_body, handler, fallthrough } = &block.terminal {
            let loc = block.instructions.last().map_or(oxc_span::SPAN, |i| i.loc);

            if has_throw_in_try_body(&block_map, *try_body, *handler, *fallthrough) {
                errors.push(CompilerError::todo(
                    loc,
                    "(BuildHIR::lowerStatement) Support ThrowStatement inside of try/catch"
                        .to_string(),
                ));
            }
        }
    }
}

/// Walk blocks reachable from `start` (within the try body) looking for Throw terminals.
fn has_throw_in_try_body(
    block_map: &rustc_hash::FxHashMap<BlockId, &crate::hir::types::BasicBlock>,
    start: BlockId,
    handler: BlockId,
    fallthrough: BlockId,
) -> bool {
    let mut visited = rustc_hash::FxHashSet::default();
    let mut stack = vec![start];
    while let Some(block_id) = stack.pop() {
        if block_id == handler || block_id == fallthrough {
            continue;
        }
        if !visited.insert(block_id) {
            continue;
        }
        let Some(block) = block_map.get(&block_id) else {
            continue;
        };
        if matches!(&block.terminal, Terminal::Throw { .. }) {
            return true;
        }
        collect_terminal_successors(&block.terminal, &mut stack);
    }
    false
}

#[expect(dead_code)]
fn check_value_blocks_in_try(hir: &HIR, errors: &mut ErrorCollector) {
    // Build a lookup table for O(1) block access instead of O(n) linear scan.
    let block_map: rustc_hash::FxHashMap<BlockId, &crate::hir::types::BasicBlock> =
        hir.blocks.iter().map(|(id, block)| (*id, block)).collect();

    for (_, block) in &hir.blocks {
        if let Terminal::Try { block: try_body, handler, fallthrough } = &block.terminal {
            let loc = block.instructions.last().map_or(oxc_span::SPAN, |i| i.loc);

            // Walk all blocks reachable from the try body.
            // A "value block" terminal is one that creates intermediate
            // evaluation blocks inside the try scope.
            let (has_value_block, has_throw) =
                check_try_body_terminals(&block_map, *try_body, *handler, *fallthrough);
            if has_value_block {
                errors.push(CompilerError::todo(
                    loc,
                    "Support value blocks (conditional, logical, optional chaining, etc) within a try/catch statement".to_string(),
                ));
            }
            if has_throw {
                errors.push(CompilerError::todo(
                    loc,
                    "(BuildHIR::lowerStatement) Support ThrowStatement inside of try/catch"
                        .to_string(),
                ));
            }
        }
    }
}

/// Check blocks reachable from `start` (within the try body, not entering
/// the handler or fallthrough) for "value block" terminals and throw terminals.
/// Returns (has_value_block, has_throw).
fn check_try_body_terminals(
    block_map: &rustc_hash::FxHashMap<BlockId, &crate::hir::types::BasicBlock>,
    start: BlockId,
    handler: BlockId,
    fallthrough: BlockId,
) -> (bool, bool) {
    let mut visited = rustc_hash::FxHashSet::default();
    let mut stack = vec![start];
    let mut found_value_block = false;
    let mut found_throw = false;
    while let Some(block_id) = stack.pop() {
        // Don't cross into handler or fallthrough — those are outside the try body
        if block_id == handler || block_id == fallthrough {
            continue;
        }
        if !visited.insert(block_id) {
            continue;
        }
        let Some(block) = block_map.get(&block_id) else {
            continue;
        };
        if is_value_block_terminal(&block.terminal) {
            found_value_block = true;
        }
        if matches!(&block.terminal, Terminal::Throw { .. }) {
            found_throw = true;
        }
        if found_value_block && found_throw {
            return (true, true);
        }
        // Follow successors within the try body
        // (including nested try handler blocks, which are still reachable from
        // the outer try body)
        collect_terminal_successors(&block.terminal, &mut stack);
    }
    (found_value_block, found_throw)
}

/// Returns true if the terminal creates "value blocks" that upstream doesn't support
/// inside try/catch.
fn is_value_block_terminal(terminal: &Terminal) -> bool {
    matches!(
        terminal,
        Terminal::Ternary { .. }
            | Terminal::Logical { .. }
            | Terminal::Optional { .. }
            | Terminal::For { .. }
            | Terminal::ForOf { .. }
            | Terminal::ForIn { .. }
    )
}

/// Collect all successor block IDs from a terminal.
fn collect_terminal_successors(terminal: &Terminal, successors: &mut Vec<BlockId>) {
    match terminal {
        Terminal::Goto { block } => {
            successors.push(*block);
        }
        Terminal::MaybeThrow { continuation, handler, .. } => {
            successors.push(*continuation);
            successors.push(*handler);
        }
        Terminal::If { consequent, alternate, fallthrough, .. } => {
            successors.push(*consequent);
            successors.push(*alternate);
            successors.push(*fallthrough);
        }
        Terminal::Branch { consequent, alternate, .. } => {
            successors.push(*consequent);
            successors.push(*alternate);
        }
        Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
            successors.push(*consequent);
            successors.push(*alternate);
            successors.push(*fallthrough);
        }
        Terminal::Logical { right, fallthrough, .. } => {
            successors.push(*right);
            successors.push(*fallthrough);
        }
        Terminal::Optional { consequent, fallthrough, .. } => {
            successors.push(*consequent);
            successors.push(*fallthrough);
        }
        Terminal::Switch { cases, fallthrough, .. } => {
            for case in cases {
                successors.push(case.block);
            }
            successors.push(*fallthrough);
        }
        Terminal::For { init, test, update, body, fallthrough } => {
            successors.push(*init);
            successors.push(*test);
            if let Some(u) = update {
                successors.push(*u);
            }
            successors.push(*body);
            successors.push(*fallthrough);
        }
        Terminal::ForOf { init, test, body, fallthrough }
        | Terminal::ForIn { init, test, body, fallthrough } => {
            successors.push(*init);
            successors.push(*test);
            successors.push(*body);
            successors.push(*fallthrough);
        }
        Terminal::DoWhile { body, test, fallthrough }
        | Terminal::While { test, body, fallthrough } => {
            successors.push(*test);
            successors.push(*body);
            successors.push(*fallthrough);
        }
        Terminal::Sequence { blocks, fallthrough } => {
            successors.extend(blocks);
            successors.push(*fallthrough);
        }
        Terminal::Try { block, handler, fallthrough } => {
            successors.push(*block);
            successors.push(*handler);
            successors.push(*fallthrough);
        }
        Terminal::Scope { block, fallthrough, .. }
        | Terminal::PrunedScope { block, fallthrough, .. } => {
            successors.push(*block);
            successors.push(*fallthrough);
        }
        Terminal::Label { block, fallthrough, .. } => {
            successors.push(*block);
            successors.push(*fallthrough);
        }
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => {}
    }
}

/// Check for duplicate fbt/fbs sub-tags that upstream cannot handle.
///
/// Upstream: `Todo: Support duplicate fbt tags`
/// When `fbt._()` is called with an array argument containing multiple `fbt._enum()`,
/// `fbt._plural()`, or `fbt._pronoun()` calls of the same type, upstream bails because
/// the fbt Babel plugin's deduplication logic depends on `.start`/`.end` source positions
/// that the compiler doesn't preserve for synthesized nodes.
///
/// Detection: Count MethodCall instructions on `fbt` receiver with method names
/// `_enum`, `_plural`, `_pronoun`. If any type appears 2+ times, bail.
fn check_fbt_duplicate_tags(hir: &HIR, errors: &mut ErrorCollector) {
    // Pass 1: Collect identifiers whose name is "fbt" or "fbs" (from imports or locals).
    // These are loaded via LoadLocal (imports are treated as local bindings).
    let mut fbt_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            match &instr.value {
                InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
                    if place.identifier.name.as_deref().is_some_and(|n| n == "fbt" || n == "fbs") {
                        fbt_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                InstructionValue::LoadGlobal { binding } => {
                    if binding.name == "fbt" || binding.name == "fbs" {
                        fbt_ids.insert(instr.lvalue.identifier.id);
                    }
                }
                _ => {}
            }
        }
    }
    if fbt_ids.is_empty() {
        return;
    }

    // Pass 2: Count fbt sub-tag method calls (_enum, _plural, _pronoun)
    let mut enum_count = 0u32;
    let mut plural_count = 0u32;
    let mut pronoun_count = 0u32;
    let mut first_loc = None;
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::MethodCall { receiver, property, .. } = &instr.value
                && fbt_ids.contains(&receiver.identifier.id)
            {
                match property.as_str() {
                    "_enum" => {
                        enum_count += 1;
                        if first_loc.is_none() {
                            first_loc = Some(instr.loc);
                        }
                    }
                    "_plural" => {
                        plural_count += 1;
                        if first_loc.is_none() {
                            first_loc = Some(instr.loc);
                        }
                    }
                    "_pronoun" => {
                        pronoun_count += 1;
                        if first_loc.is_none() {
                            first_loc = Some(instr.loc);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let loc = first_loc.unwrap_or_default();
    if enum_count > 1 {
        errors.push(CompilerError::todo(
            loc,
            "Support duplicate fbt tags\n\nSupport `<fbt>` tags with multiple `<fbt:enum>` values."
                .to_string(),
        ));
    }
    if plural_count > 1 {
        errors.push(CompilerError::todo(
            loc,
            "Support duplicate fbt tags\n\nSupport `<fbt>` tags with multiple `<fbt:plural>` values."
                .to_string(),
        ));
    }
    if pronoun_count > 1 {
        errors.push(CompilerError::todo(
            loc,
            "Support duplicate fbt tags\n\nSupport `<fbt>` tags with multiple `<fbt:pronoun>` values."
                .to_string(),
        ));
    }
}

/// Detect self-referencing variable declarations: `const x = identity(x)`.
///
/// In JavaScript, `const x = f(x)` is a TDZ error because `x` is referenced
/// before initialization completes. Our HIR builder emits DeclareLocal before
/// lowering the initializer, so the RHS `x` resolves to the same identifier
/// as the LHS. Upstream's SSA pass detects this as "identifier used before
/// defined" and bails with a Todo error.
///
/// Detection: for each `DeclareLocal` with `Const` kind, check if the immediately
/// following sequence loads the same identifier (by ID) before the StoreLocal.
/// Only fires when the LoadLocal ID exactly matches the DeclareLocal lvalue ID
/// (not just by name), to avoid false positives on destructured params or shadowed vars.
fn check_self_referencing_declarations(hir: &HIR, errors: &mut ErrorCollector) {
    for (_, block) in &hir.blocks {
        let instrs = &block.instructions;
        for (i, instr) in instrs.iter().enumerate() {
            if let InstructionValue::DeclareLocal { lvalue, type_: InstructionKind::Const } =
                &instr.value
            {
                let declared_id = lvalue.identifier.id;
                // Only check named identifiers (not temps)
                let Some(declared_name) = &lvalue.identifier.name else {
                    continue;
                };
                if declared_name.starts_with('t')
                    && !declared_name[1..].is_empty()
                    && declared_name[1..].chars().all(|c| c.is_ascii_digit())
                {
                    // Skip temp identifiers (t0, t1, ...) — these are compiler-generated
                    continue;
                }
                // Scan forward until we find the matching StoreLocal
                for next in &instrs[(i + 1)..] {
                    // Found the StoreLocal for this declaration — stop scanning
                    if let InstructionValue::StoreLocal { lvalue: sl, .. } = &next.value
                        && sl.identifier.id == declared_id
                    {
                        break;
                    }
                    // Also stop at DeclareLocal/Destructure for a different variable
                    // to avoid scanning too far
                    if matches!(
                        &next.value,
                        InstructionValue::DeclareLocal { .. }
                            | InstructionValue::Destructure { .. }
                    ) {
                        break;
                    }
                    // Check if any LoadLocal loads the exact same identifier ID
                    if let InstructionValue::LoadLocal { place } = &next.value
                        && place.identifier.id == declared_id
                    {
                        errors.push(CompilerError::todo(
                            next.loc,
                            format!(
                                "[hoisting] EnterSSA: Expected identifier to be defined \
                                 before being used. Identifier {declared_name} is undefined.",
                            ),
                        ));
                        return;
                    }
                }
            }
        }
    }
}

/// Check for nested MethodCall results used as arguments to other MethodCalls.
///
/// Upstream codegen fails with:
///   "Invariant: [Codegen] Internal error: MethodCall::property must be an
///    unpromoted + unmemoized MemberExpression"
/// when a MethodCall result is used as an argument to another MethodCall.
/// Examples:
///   - `Math.max(2, items.push(5), ...other)` — push() result is arg to max()
///   - `Math.floor(diff.bar())` — bar() result is arg to floor()
///
/// We detect this early and bail to match upstream behavior.
fn check_nested_method_call_as_argument(hir: &HIR, errors: &mut ErrorCollector) {
    // Pass 1: Collect all lvalue IDs that are results of MethodCall instructions
    let mut method_call_result_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if matches!(instr.value, InstructionValue::MethodCall { .. }) {
                method_call_result_ids.insert(instr.lvalue.identifier.id);
            }
        }
    }

    if method_call_result_ids.is_empty() {
        return;
    }

    // Pass 2: Check if any MethodCall has an argument whose ID is a MethodCall result
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let InstructionValue::MethodCall { args, .. } = &instr.value {
                for arg in args {
                    if method_call_result_ids.contains(&arg.identifier.id) {
                        errors.push(CompilerError::invariant(
                            instr.loc,
                            "[Codegen] Internal error: MethodCall::property must be an \
                             unpromoted + unmemoized MemberExpression"
                                .to_string(),
                        ));
                        return;
                    }
                }
            }
        }
    }
}
