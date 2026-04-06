//! Infer mutation and aliasing effects for all instructions.
//!
//! Upstream: `InferMutationAliasingEffects.ts`
//!
//! This pass implements an abstract interpreter that walks the HIR CFG and
//! determines how each instruction affects the abstract heap model. It assigns
//! refined `AliasingEffect` annotations to each instruction.
//!
//! The key data structure is `InferenceState`, which maintains:
//! - A set of abstract values, each with a `ValueKind` and set of reasons
//! - A mapping from `IdentifierId` to a set of abstract values (to handle phis)
//!
//! The pass uses a worklist-based fixpoint iteration over CFG blocks, merging
//! states at join points. For each instruction, candidate effects are computed
//! once (cached), then applied against the current abstract state to produce
//! refined effects.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::hir::types::{
    AliasingEffect, AliasingSignature, BlockId, Effect, FunctionSignature, HIR, IdentifierId,
    InstructionValue, Phi, Place, SourceLocation, Terminal, ValueKind, ValueReason,
};
use crate::inference::analyse_functions::MethodSignatures;

// ---------------------------------------------------------------------------
// AbstractValueId — surrogate for upstream's InstructionValue object identity
// ---------------------------------------------------------------------------

/// Unique identifier for an abstract value in the heap.
///
/// Upstream uses JavaScript object identity (`Map<InstructionValue, AbstractValue>`).
/// Since Rust doesn't have reference-identity maps, we use explicit IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AbstractValueId(u32);

// ---------------------------------------------------------------------------
// AbstractValue
// ---------------------------------------------------------------------------

/// The abstract kind of a value in the heap.
///
/// Upstream: `AbstractValue = { kind: ValueKind, reason: Set<ValueReason> }`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AbstractValue {
    kind: ValueKind,
    reasons: FxHashSet<ValueReason>,
}

impl AbstractValue {
    fn new(kind: ValueKind, reason: ValueReason) -> Self {
        let mut reasons = FxHashSet::default();
        reasons.insert(reason);
        Self { kind, reasons }
    }

    fn with_reasons(kind: ValueKind, reasons: FxHashSet<ValueReason>) -> Self {
        Self { kind, reasons }
    }
}

/// Merge two abstract values according to the lattice.
///
/// Upstream: `mergeAbstractValues()`.
/// Lattice order (bottom to top): Primitive < Global < Frozen < MaybeFrozen < Mutable
fn merge_abstract_values(a: &AbstractValue, b: &AbstractValue) -> AbstractValue {
    let merged_kind = merge_value_kinds(a.kind, b.kind);
    let mut reasons = a.reasons.clone();
    reasons.extend(&b.reasons);
    AbstractValue { kind: merged_kind, reasons }
}

/// Lattice join for value kinds.
fn merge_value_kinds(a: ValueKind, b: ValueKind) -> ValueKind {
    if a == b {
        return a;
    }
    use ValueKind::{Context, Frozen, Global, MaybeFrozen, Mutable, Primitive};
    match (a, b) {
        (Primitive, Global) | (Global, Primitive) => Global,
        (Primitive | Global, Frozen) | (Frozen, Primitive | Global) => Frozen,
        (Primitive | Global | Frozen, MaybeFrozen) | (MaybeFrozen, Primitive | Global | Frozen) => {
            MaybeFrozen
        }
        // Context is treated like Mutable when merging with non-Context
        (Context, Context) => Context,
        _ => Mutable,
    }
}

// ---------------------------------------------------------------------------
// InferenceState — the abstract state at a program point
// ---------------------------------------------------------------------------

/// Abstract state mapping identifiers to sets of abstract values.
///
/// Upstream: `InferenceState` class.
///
/// Key design: each identifier maps to a *set* of abstract values to handle
/// phi nodes where a variable may have different values from different paths.
/// The `kind()` method merges the kinds of all values a variable may hold.
#[derive(Debug, Clone)]
#[expect(dead_code)]
struct InferenceState {
    /// All abstract values, indexed by AbstractValueId.
    values: Vec<AbstractValue>,
    /// Maps each identifier to the set of abstract value IDs it may hold.
    variables: FxHashMap<IdentifierId, FxHashSet<AbstractValueId>>,
    /// Whether this is a function expression (affects freeze behavior on return).
    is_function_expression: bool,
}

#[expect(dead_code)]
impl InferenceState {
    fn new(is_function_expression: bool) -> Self {
        Self { values: Vec::new(), variables: FxHashMap::default(), is_function_expression }
    }

    /// Allocate a new abstract value and return its ID.
    fn alloc_value(&mut self, value: AbstractValue) -> AbstractValueId {
        let id = AbstractValueId(self.values.len() as u32);
        self.values.push(value);
        id
    }

    /// Initialize a new abstract value (upstream: `state.initialize(value, kind)`).
    /// Returns the allocated value ID.
    fn initialize(&mut self, kind: ValueKind, reason: ValueReason) -> AbstractValueId {
        self.alloc_value(AbstractValue::new(kind, reason))
    }

    /// Initialize with a full AbstractValue.
    fn initialize_value(&mut self, value: AbstractValue) -> AbstractValueId {
        self.alloc_value(value)
    }

    /// Define: set an identifier to point to exactly one abstract value.
    /// Upstream: `state.define(place, value)`.
    fn define(&mut self, place_id: IdentifierId, value_id: AbstractValueId) {
        let mut set = FxHashSet::default();
        set.insert(value_id);
        self.variables.insert(place_id, set);
    }

    /// Check if a place has been defined.
    fn is_defined(&self, place_id: IdentifierId) -> bool {
        self.variables.contains_key(&place_id)
    }

    /// Get the set of abstract value IDs for an identifier.
    fn value_ids(&self, place_id: IdentifierId) -> Option<&FxHashSet<AbstractValueId>> {
        self.variables.get(&place_id)
    }

    /// Get the merged abstract value for a place.
    ///
    /// Upstream: `state.kind(place)`. Merges all values the place may hold.
    fn kind(&self, place_id: IdentifierId) -> AbstractValue {
        let Some(value_ids) = self.variables.get(&place_id) else {
            // If not defined, default to Mutable (conservative).
            // DIVERGENCE: Upstream throws an invariant error here. We're more lenient
            // because our HIR may have identifiers not yet tracked (e.g. from phi backedges).
            return AbstractValue::new(ValueKind::Mutable, ValueReason::Other);
        };

        let mut merged: Option<AbstractValue> = None;
        for &vid in value_ids {
            let val = &self.values[vid.0 as usize];
            merged = Some(match merged {
                Some(m) => merge_abstract_values(&m, val),
                None => val.clone(),
            });
        }

        merged.unwrap_or_else(|| AbstractValue::new(ValueKind::Mutable, ValueReason::Other))
    }

    /// Assign: make `into` point to the same values as `from`.
    /// Upstream: `state.assign(place, value)`.
    fn assign(&mut self, into_id: IdentifierId, from_id: IdentifierId) {
        if let Some(values) = self.variables.get(&from_id).cloned() {
            self.variables.insert(into_id, values);
        }
    }

    /// Append alias: add the values of `from` to `into`'s value set.
    /// Upstream: `state.appendAlias(place, value)`.
    fn append_alias(&mut self, into_id: IdentifierId, from_id: IdentifierId) {
        let from_values = self.variables.get(&from_id).cloned().unwrap_or_default();
        let into_values = self.variables.entry(into_id).or_default();
        into_values.extend(from_values);
    }

    /// Freeze a place: set all its values to Frozen.
    /// Returns true if any value was actually frozen (was mutable/context/maybefrozen).
    fn freeze(&mut self, place_id: IdentifierId, reason: ValueReason) -> bool {
        let kind = self.kind(place_id);
        match kind.kind {
            ValueKind::Context | ValueKind::Mutable | ValueKind::MaybeFrozen => {
                if let Some(value_ids) = self.variables.get(&place_id).cloned() {
                    for vid in value_ids {
                        self.freeze_value(vid, reason);
                    }
                }
                true
            }
            ValueKind::Frozen | ValueKind::Global | ValueKind::Primitive => false,
        }
    }

    /// Freeze a specific abstract value.
    fn freeze_value(&mut self, value_id: AbstractValueId, reason: ValueReason) {
        let val = &mut self.values[value_id.0 as usize];
        val.kind = ValueKind::Frozen;
        val.reasons.clear();
        val.reasons.insert(reason);
    }

    /// Check mutation outcome for a place.
    ///
    /// Upstream: `state.mutate(variant, place)`.
    /// Returns the kind of mutation that would occur.
    fn mutate(&self, variant: MutateVariant, place_id: IdentifierId) -> MutateResult {
        let kind = self.kind(place_id).kind;
        match variant {
            MutateVariant::MutateConditionally | MutateVariant::MutateTransitiveConditionally => {
                match kind {
                    ValueKind::Mutable | ValueKind::Context => MutateResult::Mutate,
                    _ => MutateResult::None,
                }
            }
            MutateVariant::Mutate | MutateVariant::MutateTransitive => match kind {
                ValueKind::Mutable | ValueKind::Context => MutateResult::Mutate,
                ValueKind::Primitive => MutateResult::None,
                ValueKind::Frozen | ValueKind::MaybeFrozen => MutateResult::MutateFrozen,
                ValueKind::Global => MutateResult::MutateGlobal,
            },
        }
    }

    /// Merge another state into this one. Returns true if this state changed.
    ///
    /// Upstream: `state.merge(other)`.
    fn merge(&mut self, other: &InferenceState) -> bool {
        let mut changed = false;

        // Merge values: for values that exist in both, merge their kinds.
        // PERF: Merge in-place instead of allocating a new AbstractValue per merge.

        // Ensure our values array is large enough
        if other.values.len() > self.values.len() {
            self.values.extend_from_slice(&other.values[self.values.len()..]);
            changed = true;
        }

        // Merge kinds for shared values — in-place to avoid allocations
        for (i, other_val) in other.values.iter().enumerate() {
            if i < self.values.len() {
                let self_val = &mut self.values[i];
                let merged_kind = merge_value_kinds(self_val.kind, other_val.kind);
                if merged_kind != self_val.kind {
                    self_val.kind = merged_kind;
                    changed = true;
                }
                // Merge reasons in-place
                let before_len = self_val.reasons.len();
                self_val.reasons.extend(&other_val.reasons);
                if self_val.reasons.len() != before_len {
                    changed = true;
                }
            }
        }

        // Merge variables
        for (&id, other_vals) in &other.variables {
            if let Some(self_vals) = self.variables.get_mut(&id) {
                let before_len = self_vals.len();
                self_vals.extend(other_vals);
                if self_vals.len() != before_len {
                    changed = true;
                }
            } else {
                self.variables.insert(id, other_vals.clone());
                changed = true;
            }
        }

        changed
    }

    /// Process a phi node: the phi's place gets the union of all operand values.
    ///
    /// Upstream: `state.inferPhi(phi)`.
    fn infer_phi(&mut self, phi: &Phi) {
        let mut values = FxHashSet::default();
        for (_, operand) in &phi.operands {
            if let Some(operand_values) = self.variables.get(&operand.identifier.id) {
                values.extend(operand_values);
            }
            // If operand not defined yet (backedge), skip — will be handled on next iteration
        }
        if !values.is_empty() {
            self.variables.insert(phi.place.identifier.id, values);
        }
    }

    /// PERF: Lightweight phi processing that takes pre-extracted IDs instead of full Phi struct.
    /// Avoids cloning Phi (which contains Place with String/Box fields).
    fn infer_phi_lightweight(&mut self, place_id: IdentifierId, operand_ids: &[IdentifierId]) {
        let mut values = FxHashSet::default();
        for &operand_id in operand_ids {
            if let Some(operand_values) = self.variables.get(&operand_id) {
                values.extend(operand_values);
            }
        }
        if !values.is_empty() {
            self.variables.insert(place_id, values);
        }
    }
}

/// Mutation variant for `InferenceState::mutate()`.
#[derive(Debug, Clone, Copy)]
enum MutateVariant {
    Mutate,
    MutateConditionally,
    MutateTransitive,
    MutateTransitiveConditionally,
}

/// Result of attempting a mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MutateResult {
    None,
    Mutate,
    MutateFrozen,
    MutateGlobal,
}

// ---------------------------------------------------------------------------
// Context — caching and metadata for the inference pass
// ---------------------------------------------------------------------------

/// Caching context for the inference pass.
///
/// Upstream: `Context` class.
struct InferenceContext {
    /// Whether the function being analyzed is a function expression.
    is_function_expression: bool,
    /// Cached instruction signatures: (block_index, instr_index) -> effects.
    instruction_signature_cache: FxHashMap<(usize, usize), Vec<AliasingEffect>>,
    /// Maps catch handler block IDs to the catch binding identifier.
    /// Upstream: `context.catchHandlers`.
    /// Built by scanning Try terminals to find handler blocks and their bindings.
    catch_handlers: FxHashMap<BlockId, Place>,
}

impl InferenceContext {
    fn new(is_function_expression: bool, catch_handlers: FxHashMap<BlockId, Place>) -> Self {
        Self {
            is_function_expression,
            instruction_signature_cache: FxHashMap::default(),
            catch_handlers,
        }
    }
}

/// Build a map from catch handler block IDs to their catch binding places.
///
/// Upstream: built during `inferBlock()` for Try terminals.
/// We scan all Try terminals in the HIR to find handler blocks,
/// then find the first DeclareLocal in the handler block as the catch binding.
fn build_catch_handlers(hir: &HIR) -> FxHashMap<BlockId, Place> {
    let mut catch_handlers = FxHashMap::default();

    // Collect all handler block IDs from Try terminals
    for (_, block) in &hir.blocks {
        if let Terminal::Try { handler, .. } = &block.terminal {
            // Find the catch binding in the handler block: the first DeclareLocal
            if let Some((_, handler_block)) = hir.blocks.iter().find(|(id, _)| *id == *handler) {
                for instr in &handler_block.instructions {
                    if let InstructionValue::DeclareLocal { lvalue, .. } = &instr.value {
                        catch_handlers.insert(*handler, lvalue.clone());
                        break;
                    }
                }
            }
        }
    }

    catch_handlers
}

// ---------------------------------------------------------------------------
// Public entry points
// ---------------------------------------------------------------------------

/// Infer mutation and aliasing effects for all instructions.
///
/// Upstream: `inferMutationAliasingEffects()`.
#[expect(clippy::implicit_hasher)]
pub fn infer_mutation_aliasing_effects(
    hir: &mut HIR,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    method_signatures: &MethodSignatures,
) {
    infer_mutation_aliasing_effects_inner(hir, fn_signatures, method_signatures, &[], &[], false);
}

/// Variant that accepts parameter names for pre-freezing.
#[expect(clippy::implicit_hasher)]
pub fn infer_mutation_aliasing_effects_with_params(
    hir: &mut HIR,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    method_signatures: &MethodSignatures,
    param_names: &[String],
) {
    infer_mutation_aliasing_effects_inner(
        hir,
        fn_signatures,
        method_signatures,
        param_names,
        &[],
        false,
    );
}

fn infer_mutation_aliasing_effects_inner(
    hir: &mut HIR,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    method_signatures: &MethodSignatures,
    param_names: &[String],
    _param_ids: &[IdentifierId],
    is_function_expression: bool,
) {
    let mut state = InferenceState::new(is_function_expression);
    let catch_handlers = build_catch_handlers(hir);
    let mut ctx = InferenceContext::new(is_function_expression, catch_handlers);

    // Initialize parameters in the abstract state.
    // Component/hook params are frozen; function expression params are mutable.
    let param_kind = if is_function_expression { ValueKind::Mutable } else { ValueKind::Frozen };
    let param_reason = if is_function_expression {
        ValueReason::Other
    } else {
        ValueReason::ReactiveFunctionArgument
    };

    // Pre-freeze named parameters (component props).
    // DIVERGENCE: Upstream initializes params from the HIRFunction.params list.
    // We do the same via param_names since our pipeline passes those separately.
    if !param_names.is_empty() {
        pre_freeze_params(hir, &mut state, param_names, param_kind, param_reason);
    }

    // Build block index for fast lookup
    let block_ids: Vec<BlockId> = hir.blocks.iter().map(|(id, _)| *id).collect();
    let block_index: FxHashMap<BlockId, usize> =
        block_ids.iter().enumerate().map(|(i, &id)| (id, i)).collect();

    // Worklist-based fixpoint iteration over CFG blocks.
    // Upstream: iterates `queuedStates` map until empty.
    let mut queued_states: FxHashMap<BlockId, InferenceState> = FxHashMap::default();
    let mut states_by_block: FxHashMap<BlockId, InferenceState> = FxHashMap::default();

    // Queue the entry block with the initial state.
    queued_states.insert(hir.entry, state);

    // PERF: Reuse a sorted buffer across iterations to avoid per-iteration allocation.
    let mut sorted_queue: Vec<(usize, BlockId, InferenceState)> = Vec::new();
    // PERF: Reuse successor buffer to avoid per-block Vec allocation.
    let mut successor_buf: Vec<BlockId> = Vec::with_capacity(8);

    let mut iteration_count = 0;
    while !queued_states.is_empty() {
        iteration_count += 1;
        if iteration_count > 100 {
            // Upstream throws an invariant error. We just break to avoid infinite loops.
            break;
        }

        // PERF: Drain into a reusable buffer and sort by block index for program-order
        // processing. This improves convergence speed (forward dataflow converges faster
        // when processed in forward order) and avoids allocating a new Vec each iteration.
        sorted_queue.clear();
        for (block_id, incoming_state) in queued_states.drain() {
            if let Some(&block_idx) = block_index.get(&block_id) {
                sorted_queue.push((block_idx, block_id, incoming_state));
            }
        }
        sorted_queue.sort_unstable_by_key(|(idx, _, _)| *idx);

        // PERF: drain(..) reuses the Vec's allocation across iterations (unlike into_iter()).
        #[expect(clippy::iter_with_drain)]
        for (block_idx, block_id, incoming_state) in sorted_queue.drain(..) {
            // PERF: Move the incoming state directly into block_state instead of cloning.
            // We save a reference copy in states_by_block for the merge check in queue_state,
            // but use the moved original for block processing.
            states_by_block.insert(block_id, incoming_state.clone());
            let mut block_state = incoming_state;

            // Process the block: phis, instructions, terminal
            infer_block(
                &mut ctx,
                &mut block_state,
                hir,
                block_idx,
                fn_signatures,
                method_signatures,
            );

            // Queue successor blocks
            // PERF: Reuse successor buffer instead of allocating Vec per block.
            successor_buf.clear();
            terminal_successors_into(&hir.blocks[block_idx].1.terminal, &mut successor_buf);
            for &succ_id in &successor_buf {
                queue_state(&mut queued_states, &states_by_block, succ_id, &block_state);
            }
        }
    }

    // Final pass: write Place.effect for all operands based on the final state.
    // We use the states_by_block to find the state at each block.
    write_place_effects(hir, &states_by_block, &block_index);
}

/// Queue a state for a block, merging with existing queued/previous states.
fn queue_state(
    queued_states: &mut FxHashMap<BlockId, InferenceState>,
    states_by_block: &FxHashMap<BlockId, InferenceState>,
    block_id: BlockId,
    state: &InferenceState,
) {
    if let Some(queued) = queued_states.get_mut(&block_id) {
        queued.merge(state);
    } else if let Some(prev) = states_by_block.get(&block_id) {
        let mut next = prev.clone();
        if next.merge(state) {
            queued_states.insert(block_id, next);
        }
    } else {
        queued_states.insert(block_id, state.clone());
    }
}

// ---------------------------------------------------------------------------
// Pre-freeze parameters
// ---------------------------------------------------------------------------

/// Pre-freeze function parameters in the abstract state.
///
/// DIVERGENCE: Upstream initializes params directly from `fn.params`. We walk
/// all instructions to find places matching param names, since our HIR creates
/// fresh IdentifierIds per Place reference.
fn pre_freeze_params(
    hir: &HIR,
    state: &mut InferenceState,
    param_names: &[String],
    kind: ValueKind,
    reason: ValueReason,
) {
    // Collect all identifier IDs that correspond to parameter names
    let mut param_ids_seen = FxHashSet::default();

    let freeze_if_param =
        |place: &Place, state: &mut InferenceState, seen: &mut FxHashSet<IdentifierId>| {
            if let Some(name) = &place.identifier.name
                && param_names.iter().any(|p| p == name)
                && seen.insert(place.identifier.id)
            {
                let vid = state.initialize(kind, reason);
                state.define(place.identifier.id, vid);
            }
        };

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            freeze_if_param(&instr.lvalue, state, &mut param_ids_seen);
            visit_instruction_operands(&instr.value, &mut |place| {
                freeze_if_param(place, state, &mut param_ids_seen);
            });
        }
    }
}

/// Visit all operand places in an instruction value.
fn visit_instruction_operands(value: &InstructionValue, visitor: &mut dyn FnMut(&Place)) {
    use crate::hir::types::{ArrayElement, ObjectPropertyKey};

    match value {
        InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::TypeCastExpression { value: place, .. }
        | InstructionValue::UnaryExpression { value: place, .. }
        | InstructionValue::PostfixUpdate { lvalue: place, .. }
        | InstructionValue::PrefixUpdate { lvalue: place, .. }
        | InstructionValue::Await { value: place }
        | InstructionValue::GetIterator { collection: place }
        | InstructionValue::NextPropertyOf { value: place }
        | InstructionValue::StoreGlobal { value: place, .. } => {
            visitor(place);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            visitor(lvalue);
            visitor(value);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            visitor(lvalue);
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            visitor(callee);
            for arg in args {
                visitor(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            visitor(receiver);
            for arg in args {
                visitor(arg);
            }
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            visitor(left);
            visitor(right);
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            visitor(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            visitor(object);
            visitor(value);
        }
        InstructionValue::ComputedLoad { object, property, .. }
        | InstructionValue::ComputedDelete { object, property } => {
            visitor(object);
            visitor(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            visitor(object);
            visitor(property);
            visitor(value);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                if let ObjectPropertyKey::Computed(p) = &prop.key {
                    visitor(p);
                }
                visitor(&prop.value);
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for el in elements {
                match el {
                    ArrayElement::Expression(p) | ArrayElement::Spread(p) => visitor(p),
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            visitor(tag);
            for prop in props {
                visitor(&prop.value);
            }
            for child in children {
                visitor(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                visitor(child);
            }
        }
        InstructionValue::Destructure { value, .. } => {
            visitor(value);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            visitor(iterator);
        }
        InstructionValue::TaggedTemplateExpression { tag, value: tagged_value, .. } => {
            visitor(tag);
            for expr in &tagged_value.subexpressions {
                visitor(expr);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for expr in subexpressions {
                visitor(expr);
            }
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            visitor(decl);
            for dep in deps {
                visitor(dep);
            }
        }
        InstructionValue::FunctionExpression { lowered_func, .. } => {
            for ctx_place in &lowered_func.context {
                visitor(ctx_place);
            }
        }
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// inferBlock — process one basic block
// ---------------------------------------------------------------------------

/// Process a single basic block: phis, instructions, terminal effects.
///
/// Upstream: `inferBlock()`.
fn infer_block(
    ctx: &mut InferenceContext,
    state: &mut InferenceState,
    hir: &mut HIR,
    block_idx: usize,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    method_signatures: &MethodSignatures,
) {
    // PERF: Process phi nodes without cloning the full Phi structs.
    // We only need the place id and operand ids for state.infer_phi().
    // Extract the lightweight data first, then process against the state.
    let phi_data: Vec<(IdentifierId, Vec<IdentifierId>)> = hir.blocks[block_idx]
        .1
        .phis
        .iter()
        .map(|phi| {
            let operand_ids: Vec<IdentifierId> =
                phi.operands.iter().map(|(_, op)| op.identifier.id).collect();
            (phi.place.identifier.id, operand_ids)
        })
        .collect();
    for (place_id, operand_ids) in &phi_data {
        state.infer_phi_lightweight(*place_id, operand_ids);
    }

    // Process instructions
    let num_instrs = hir.blocks[block_idx].1.instructions.len();
    for instr_idx in 0..num_instrs {
        // Get or compute the candidate signature for this instruction.
        // PERF: Compute once and cache. The clone is needed because apply_signature
        // takes &mut ctx (for catch_handlers/is_function_expression access).
        let cache_key = (block_idx, instr_idx);
        let signature = if let Some(sig) = ctx.instruction_signature_cache.get(&cache_key) {
            sig.clone()
        } else {
            let sig = compute_signature_for_instruction(
                &hir.blocks[block_idx].1.instructions[instr_idx],
                fn_signatures,
                method_signatures,
            );
            ctx.instruction_signature_cache.insert(cache_key, sig.clone());
            sig
        };

        // Apply the signature to produce refined effects
        let effects = apply_signature(ctx, state, &signature, block_idx, instr_idx, hir);

        // Store refined effects on the instruction
        hir.blocks[block_idx].1.instructions[instr_idx].effects =
            if effects.is_empty() { None } else { Some(effects) };
    }

    // Process terminal effects
    apply_terminal_effects(ctx, state, hir, block_idx);
}

// ---------------------------------------------------------------------------
// computeSignatureForInstruction — generate candidate effects
// ---------------------------------------------------------------------------

/// Compute the candidate aliasing effects for an instruction.
///
/// Upstream: `computeSignatureForInstruction()`.
///
/// This uses the existing `compute_instruction_effects` from `aliasing_effects.rs`
/// which already generates the correct candidate effects matching upstream.
fn compute_signature_for_instruction(
    instr: &crate::hir::types::Instruction,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    method_signatures: &MethodSignatures,
) -> Vec<AliasingEffect> {
    super::aliasing_effects::compute_instruction_effects(
        &instr.value,
        &instr.lvalue,
        instr.loc,
        fn_signatures,
        method_signatures,
    )
}

// ---------------------------------------------------------------------------
// applySignature — apply candidate effects against state
// ---------------------------------------------------------------------------

/// Apply a set of candidate effects to the state, producing refined effects.
///
/// Upstream: `applySignature()` + inner `applyEffect()` calls.
fn apply_signature(
    ctx: &mut InferenceContext,
    state: &mut InferenceState,
    signature: &[AliasingEffect],
    block_idx: usize,
    instr_idx: usize,
    hir: &HIR,
) -> Vec<AliasingEffect> {
    let mut effects = Vec::new();
    let mut initialized = FxHashSet::default();

    for (effect_idx, effect) in signature.iter().enumerate() {
        apply_effect(
            ctx,
            state,
            effect,
            &mut initialized,
            &mut effects,
            block_idx,
            instr_idx,
            effect_idx,
            hir,
        );
    }

    // Ensure the instruction lvalue is defined in the state.
    // If it wasn't initialized by the effects (e.g. StartMemoize), create a default.
    let lvalue_id = hir.blocks[block_idx].1.instructions[instr_idx].lvalue.identifier.id;
    if !state.is_defined(lvalue_id) {
        let vid = state.initialize(ValueKind::Primitive, ValueReason::Other);
        state.define(lvalue_id, vid);
    }

    effects
}

// ---------------------------------------------------------------------------
// applyEffect — the core refinement logic
// ---------------------------------------------------------------------------

/// Apply a single effect to the abstract state, refining it based on value kinds.
///
/// Upstream: `applyEffect()`.
///
/// This is the heart of the abstract interpreter. Each effect is checked against
/// the current abstract state and may be:
/// - Passed through unchanged
/// - Downgraded (e.g. Capture on frozen → ImmutableCapture)
/// - Dropped (e.g. MutateConditionally on frozen → no-op)
/// - Upgraded to an error (e.g. Mutate on frozen → MutateFrozen)
#[expect(clippy::too_many_arguments)]
fn apply_effect(
    ctx: &mut InferenceContext,
    state: &mut InferenceState,
    effect: &AliasingEffect,
    initialized: &mut FxHashSet<IdentifierId>,
    effects: &mut Vec<AliasingEffect>,
    block_idx: usize,
    instr_idx: usize,
    _effect_idx: usize,
    hir: &HIR,
) {
    match effect {
        AliasingEffect::Create { into, value, reason } => {
            initialized.insert(into.identifier.id);
            let vid = state.initialize(*value, *reason);
            state.define(into.identifier.id, vid);
            effects.push(effect.clone());
        }

        AliasingEffect::CreateFrom { from, into } => {
            initialized.insert(into.identifier.id);

            let from_value = state.kind(from.identifier.id);
            let vid = state.initialize_value(from_value.clone());
            state.define(into.identifier.id, vid);

            match from_value.kind {
                ValueKind::Primitive | ValueKind::Global => {
                    let reason =
                        from_value.reasons.iter().next().copied().unwrap_or(ValueReason::Other);
                    effects.push(AliasingEffect::Create {
                        into: into.clone(),
                        value: from_value.kind,
                        reason,
                    });
                }
                ValueKind::Frozen => {
                    let reason =
                        from_value.reasons.iter().next().copied().unwrap_or(ValueReason::Other);
                    effects.push(AliasingEffect::Create {
                        into: into.clone(),
                        value: ValueKind::Frozen,
                        reason,
                    });
                    apply_effect(
                        ctx,
                        state,
                        &AliasingEffect::ImmutableCapture {
                            from: from.clone(),
                            into: into.clone(),
                        },
                        initialized,
                        effects,
                        block_idx,
                        instr_idx,
                        0,
                        hir,
                    );
                }
                _ => {
                    effects.push(effect.clone());
                }
            }
        }

        AliasingEffect::CreateFunction { captures, function: _, into } => {
            initialized.insert(into.identifier.id);

            let has_mutable_captures = captures.iter().any(|cap| {
                let kind = state.kind(cap.identifier.id).kind;
                matches!(kind, ValueKind::Mutable | ValueKind::Context)
            });

            let fn_kind = if has_mutable_captures { ValueKind::Mutable } else { ValueKind::Frozen };
            let vid =
                state.initialize_value(AbstractValue::with_reasons(fn_kind, FxHashSet::default()));
            state.define(into.identifier.id, vid);

            effects.push(effect.clone());

            for capture in captures {
                apply_effect(
                    ctx,
                    state,
                    &AliasingEffect::Capture { from: capture.clone(), into: into.clone() },
                    initialized,
                    effects,
                    block_idx,
                    instr_idx,
                    0,
                    hir,
                );
            }
        }

        AliasingEffect::Assign { from, into } => {
            initialized.insert(into.identifier.id);

            let from_value = state.kind(from.identifier.id);
            match from_value.kind {
                ValueKind::Frozen => {
                    apply_effect(
                        ctx,
                        state,
                        &AliasingEffect::ImmutableCapture {
                            from: from.clone(),
                            into: into.clone(),
                        },
                        initialized,
                        effects,
                        block_idx,
                        instr_idx,
                        0,
                        hir,
                    );
                    let vid = state.initialize_value(from_value);
                    state.define(into.identifier.id, vid);
                }
                ValueKind::Global | ValueKind::Primitive => {
                    let vid = state.initialize_value(from_value);
                    state.define(into.identifier.id, vid);
                }
                _ => {
                    state.assign(into.identifier.id, from.identifier.id);
                    effects.push(effect.clone());
                }
            }
        }

        AliasingEffect::Alias { from, into }
        | AliasingEffect::MaybeAlias { from, into }
        | AliasingEffect::Capture { from, into } => {
            let from_kind = state.kind(from.identifier.id).kind;
            let into_kind = state.kind(into.identifier.id).kind;

            // Determine source type
            let source_type = match from_kind {
                ValueKind::Context => Some(SourceType::Context),
                ValueKind::Global | ValueKind::Primitive => None, // skip
                ValueKind::Frozen | ValueKind::MaybeFrozen => Some(SourceType::Frozen),
                ValueKind::Mutable => Some(SourceType::Mutable),
            };

            // Determine destination type
            let dest_type = match into_kind {
                ValueKind::Context => Some(DestType::Context),
                ValueKind::Mutable | ValueKind::MaybeFrozen => Some(DestType::Mutable),
                _ => None,
            };

            if let Some(source) = source_type {
                match source {
                    SourceType::Frozen => {
                        // Frozen source → ImmutableCapture
                        apply_effect(
                            ctx,
                            state,
                            &AliasingEffect::ImmutableCapture {
                                from: from.clone(),
                                into: into.clone(),
                            },
                            initialized,
                            effects,
                            block_idx,
                            instr_idx,
                            0,
                            hir,
                        );
                    }
                    SourceType::Mutable | SourceType::Context => {
                        if matches!(effect, AliasingEffect::MaybeAlias { .. }) {
                            // MaybeAlias always passes through
                            effects.push(effect.clone());
                        } else if source == SourceType::Mutable
                            && dest_type == Some(DestType::Mutable)
                        {
                            effects.push(effect.clone());
                        } else if (source == SourceType::Context && dest_type.is_some())
                            || (source == SourceType::Mutable
                                && dest_type == Some(DestType::Context))
                        {
                            // Context interaction → upgrade to MaybeAlias
                            apply_effect(
                                ctx,
                                state,
                                &AliasingEffect::MaybeAlias {
                                    from: from.clone(),
                                    into: into.clone(),
                                },
                                initialized,
                                effects,
                                block_idx,
                                instr_idx,
                                0,
                                hir,
                            );
                        }
                        // else: source is mutable but dest is not mutable/context → no effect
                    }
                }
            }
            // If source_type is None (primitive/global), effect is dropped
        }

        AliasingEffect::ImmutableCapture { from, .. } => {
            let kind = state.kind(from.identifier.id).kind;
            if !matches!(kind, ValueKind::Global | ValueKind::Primitive) {
                effects.push(effect.clone());
            }
        }

        AliasingEffect::Freeze { value, reason } => {
            let did_freeze = state.freeze(value.identifier.id, *reason);
            if did_freeze {
                effects.push(effect.clone());
            }
        }

        AliasingEffect::Apply {
            receiver,
            function,
            mutates_function,
            args,
            into,
            signature,
            loc,
        } => {
            apply_call_effect(
                ctx,
                state,
                receiver,
                function,
                *mutates_function,
                args,
                into,
                signature.as_ref(),
                *loc,
                initialized,
                effects,
                block_idx,
                instr_idx,
                hir,
            );
        }

        AliasingEffect::Mutate { value, reason: _ } => {
            let mutation_result = state.mutate(MutateVariant::Mutate, value.identifier.id);
            match mutation_result {
                MutateResult::Mutate => {
                    effects.push(effect.clone());
                }
                MutateResult::MutateFrozen => {
                    let abs_val = state.kind(value.identifier.id);
                    let reason_str = get_write_error_reason(&abs_val);
                    effects.push(AliasingEffect::MutateFrozen {
                        place: value.clone(),
                        error: reason_str,
                    });
                }
                MutateResult::MutateGlobal => {
                    effects.push(AliasingEffect::MutateGlobal {
                        place: value.clone(),
                        error: "Cannot mutate a global value during render".to_string(),
                    });
                }
                MutateResult::None => {}
            }
        }

        AliasingEffect::MutateConditionally { value } => {
            let mutation_result =
                state.mutate(MutateVariant::MutateConditionally, value.identifier.id);
            if mutation_result == MutateResult::Mutate {
                effects.push(effect.clone());
            }
            // Conditional on non-mutable → no-op (no error)
        }

        AliasingEffect::MutateTransitive { value } => {
            let mutation_result =
                state.mutate(MutateVariant::MutateTransitive, value.identifier.id);
            match mutation_result {
                MutateResult::Mutate => {
                    effects.push(effect.clone());
                }
                MutateResult::MutateFrozen => {
                    let abs_val = state.kind(value.identifier.id);
                    let reason_str = get_write_error_reason(&abs_val);
                    effects.push(AliasingEffect::MutateFrozen {
                        place: value.clone(),
                        error: reason_str,
                    });
                }
                MutateResult::MutateGlobal => {
                    effects.push(AliasingEffect::MutateGlobal {
                        place: value.clone(),
                        error: "Cannot mutate a global value during render".to_string(),
                    });
                }
                MutateResult::None => {}
            }
        }

        AliasingEffect::MutateTransitiveConditionally { value } => {
            let mutation_result =
                state.mutate(MutateVariant::MutateTransitiveConditionally, value.identifier.id);
            if mutation_result == MutateResult::Mutate {
                effects.push(effect.clone());
            }
        }

        // Pass-through effects (diagnostics, render markers)
        AliasingEffect::MutateFrozen { .. }
        | AliasingEffect::MutateGlobal { .. }
        | AliasingEffect::Impure { .. }
        | AliasingEffect::Render { .. } => {
            effects.push(effect.clone());
        }
    }
}

/// Source type classification for Alias/Capture/MaybeAlias refinement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceType {
    Frozen,
    Mutable,
    Context,
}

/// Destination type classification for Alias/Capture/MaybeAlias refinement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestType {
    Mutable,
    Context,
}

// ---------------------------------------------------------------------------
// Apply effect resolution (function call handling)
// ---------------------------------------------------------------------------

/// Handle an Apply effect: resolve function call through multiple strategies.
///
/// Upstream: the `Apply` case in `applyEffect()`.
///
/// Resolution order:
/// 1. Local function expression with known `aliasingEffects` → use `computeEffectsForSignature`
/// 2. Legacy `FunctionSignature` → use `computeEffectsForLegacySignature`
/// 3. Conservative fallback → assume all operands conditionally mutated
#[expect(clippy::too_many_arguments)]
fn apply_call_effect(
    ctx: &mut InferenceContext,
    state: &mut InferenceState,
    receiver: &Place,
    function: &Place,
    mutates_function: bool,
    args: &[Place],
    into: &Place,
    signature: Option<&FunctionSignature>,
    loc: SourceLocation,
    initialized: &mut FxHashSet<IdentifierId>,
    effects: &mut Vec<AliasingEffect>,
    block_idx: usize,
    instr_idx: usize,
    hir: &HIR,
) {
    // Step 1: Try to resolve via local function expression with known aliasing effects.
    // Upstream: checks if the function value is a FunctionExpression with aliasingEffects.
    if let Some(aliasing_sig) = try_resolve_function_expression(state, function, hir)
        && let Some(sig_effects) =
            compute_effects_for_signature(&aliasing_sig, receiver, args, into, loc)
    {
        for se in &sig_effects {
            apply_effect(ctx, state, se, initialized, effects, block_idx, instr_idx, 0, hir);
        }
        // If mutates_function, apply MutateConditionally on the function
        if mutates_function {
            apply_effect(
                ctx,
                state,
                &AliasingEffect::MutateConditionally { value: function.clone() },
                initialized,
                effects,
                block_idx,
                instr_idx,
                0,
                hir,
            );
        }
        return;
    }

    // Step 2: Try legacy FunctionSignature resolution
    if let Some(sig) = signature {
        let legacy_effects =
            compute_effects_for_legacy_signature(state, sig, into, receiver, args, loc);
        for le in &legacy_effects {
            apply_effect(ctx, state, le, initialized, effects, block_idx, instr_idx, 0, hir);
        }
        return;
    }

    // Step 3: No signature — conservative fallback
    // 1. Create mutable return value
    apply_effect(
        ctx,
        state,
        &AliasingEffect::Create {
            into: into.clone(),
            value: ValueKind::Mutable,
            reason: ValueReason::Other,
        },
        initialized,
        effects,
        block_idx,
        instr_idx,
        0,
        hir,
    );

    // Build operand list: receiver + function + args (always include function).
    // Upstream iterates [receiver, function, ...args] and only skips
    // MutateTransitiveConditionally for function when !mutatesFunction.
    // MaybeAlias and cross-arg Capture are applied to ALL operands.
    let mut operands: Vec<Place> = Vec::new();
    operands.push(receiver.clone());
    operands.push(function.clone());
    for arg in args {
        operands.push(arg.clone());
    }

    // 2. MaybeAlias each operand to the return value (edges before mutations)
    for operand in &operands {
        apply_effect(
            ctx,
            state,
            &AliasingEffect::MaybeAlias { from: operand.clone(), into: into.clone() },
            initialized,
            effects,
            block_idx,
            instr_idx,
            0,
            hir,
        );
    }

    // 3. Cross-arg capture: each operand may be captured into each other
    for (i, operand) in operands.iter().enumerate() {
        for (j, other) in operands.iter().enumerate() {
            if i == j {
                continue;
            }
            apply_effect(
                ctx,
                state,
                &AliasingEffect::Capture { from: operand.clone(), into: other.clone() },
                initialized,
                effects,
                block_idx,
                instr_idx,
                0,
                hir,
            );
        }
    }

    // 4. MutateTransitiveConditionally each operand (skip function if !mutates_function)
    for operand in &operands {
        if !mutates_function && operand.identifier.id == function.identifier.id {
            continue;
        }
        apply_effect(
            ctx,
            state,
            &AliasingEffect::MutateTransitiveConditionally { value: operand.clone() },
            initialized,
            effects,
            block_idx,
            instr_idx,
            0,
            hir,
        );
    }
}

// ---------------------------------------------------------------------------
// Function expression resolution
// ---------------------------------------------------------------------------

/// Try to resolve the function value to a local function expression with
/// known aliasing effects.
///
/// Upstream: in `applyEffect()`, checks `state.values(effect.function)` to find
/// a FunctionExpression whose `loweredFunc.func.aliasingEffects` is set.
///
/// We approximate this by checking if the function identifier was defined by
/// a FunctionExpression instruction in the current HIR whose `aliasing_effects`
/// has been computed.
fn try_resolve_function_expression(
    _state: &InferenceState,
    function: &Place,
    hir: &HIR,
) -> Option<AliasingSignature> {
    let fn_id = function.identifier.id;

    // Walk all instructions to find the FunctionExpression that defines this identifier.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if instr.lvalue.identifier.id != fn_id {
                continue;
            }
            if let InstructionValue::FunctionExpression { lowered_func, .. } = &instr.value {
                // Check if this FE has computed aliasing effects
                if let Some(ref aliasing_effects) = lowered_func.aliasing_effects {
                    return Some(build_signature_from_function_expression(
                        lowered_func,
                        aliasing_effects,
                    ));
                }
            }
        }
    }
    None
}

/// Build an `AliasingSignature` from a local function expression's params and effects.
///
/// Upstream: `buildSignatureFromFunctionExpression()`.
fn build_signature_from_function_expression(
    func: &crate::hir::types::HIRFunction,
    aliasing_effects: &[AliasingEffect],
) -> AliasingSignature {
    let mut params = Vec::new();
    let mut rest = None;

    for param in &func.params {
        match param {
            crate::hir::types::Param::Identifier(place) => {
                params.push(place.identifier.id);
            }
            crate::hir::types::Param::Spread(place) => {
                rest = Some(place.identifier.id);
            }
        }
    }

    AliasingSignature {
        receiver: IdentifierId(0), // FE calls don't have a meaningful receiver
        params,
        rest,
        returns: func.returns.place.identifier.id,
        effects: aliasing_effects.to_vec(),
        temporaries: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// AliasingSignature resolution
// ---------------------------------------------------------------------------

/// Compute concrete effects by substituting abstract identifiers in an
/// `AliasingSignature` with actual call-site arguments.
///
/// Upstream: `computeEffectsForSignature()`.
///
/// Returns `None` if argument count doesn't match (fallback to conservative).
fn compute_effects_for_signature(
    signature: &AliasingSignature,
    receiver: &Place,
    args: &[Place],
    into: &Place,
    loc: SourceLocation,
) -> Option<Vec<AliasingEffect>> {
    // Build substitution map: abstract IdentifierId -> concrete Place(s)
    let mut substitutions: FxHashMap<IdentifierId, Vec<Place>> = FxHashMap::default();

    // Receiver
    substitutions.insert(signature.receiver, vec![receiver.clone()]);

    // Return value
    substitutions.insert(signature.returns, vec![into.clone()]);

    // Positional params
    for (i, &param_id) in signature.params.iter().enumerate() {
        if i < args.len() {
            substitutions.insert(param_id, vec![args[i].clone()]);
        } else {
            // Fewer args than params — some params unresolved
            // This is okay; they just won't participate in effects
            substitutions.insert(param_id, Vec::new());
        }
    }

    // Rest parameter: collects remaining args
    if let Some(rest_id) = signature.rest {
        let rest_args: Vec<Place> = args.iter().skip(signature.params.len()).cloned().collect();
        substitutions.insert(rest_id, rest_args);
    }

    // Temporaries: each gets mapped to an empty list (they are created during effect application)
    for temp in &signature.temporaries {
        substitutions.insert(temp.identifier.id, vec![temp.clone()]);
    }

    // Substitute effects
    let mut result_effects = Vec::new();
    for effect in &signature.effects {
        substitute_effect(effect, &substitutions, &mut result_effects, loc);
    }

    Some(result_effects)
}

/// Substitute abstract identifiers in a single effect with concrete places.
///
/// Upstream: the substitution loop inside `computeEffectsForSignature()`.
fn substitute_effect(
    effect: &AliasingEffect,
    subs: &FxHashMap<IdentifierId, Vec<Place>>,
    out: &mut Vec<AliasingEffect>,
    loc: SourceLocation,
) {
    match effect {
        AliasingEffect::Create { into, value, reason } => {
            if let Some(places) = subs.get(&into.identifier.id) {
                for place in places {
                    out.push(AliasingEffect::Create {
                        into: place.clone(),
                        value: *value,
                        reason: *reason,
                    });
                }
            }
        }
        AliasingEffect::CreateFrom { from, into } => {
            let from_places = subs.get(&from.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for fp in &from_places {
                for ip in &into_places {
                    out.push(AliasingEffect::CreateFrom { from: fp.clone(), into: ip.clone() });
                }
            }
        }
        AliasingEffect::Assign { from, into } => {
            let from_places = subs.get(&from.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for fp in &from_places {
                for ip in &into_places {
                    out.push(AliasingEffect::Assign { from: fp.clone(), into: ip.clone() });
                }
            }
        }
        AliasingEffect::Alias { from, into } => {
            let from_places = subs.get(&from.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for fp in &from_places {
                for ip in &into_places {
                    out.push(AliasingEffect::Alias { from: fp.clone(), into: ip.clone() });
                }
            }
        }
        AliasingEffect::Capture { from, into } => {
            let from_places = subs.get(&from.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for fp in &from_places {
                for ip in &into_places {
                    out.push(AliasingEffect::Capture { from: fp.clone(), into: ip.clone() });
                }
            }
        }
        AliasingEffect::MaybeAlias { from, into } => {
            let from_places = subs.get(&from.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for fp in &from_places {
                for ip in &into_places {
                    out.push(AliasingEffect::MaybeAlias { from: fp.clone(), into: ip.clone() });
                }
            }
        }
        AliasingEffect::ImmutableCapture { from, into } => {
            let from_places = subs.get(&from.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for fp in &from_places {
                for ip in &into_places {
                    out.push(AliasingEffect::ImmutableCapture {
                        from: fp.clone(),
                        into: ip.clone(),
                    });
                }
            }
        }
        AliasingEffect::Freeze { value, reason } => {
            if let Some(places) = subs.get(&value.identifier.id) {
                for place in places {
                    out.push(AliasingEffect::Freeze { value: place.clone(), reason: *reason });
                }
            }
        }
        AliasingEffect::Mutate { value, reason } => {
            if let Some(places) = subs.get(&value.identifier.id) {
                for place in places {
                    out.push(AliasingEffect::Mutate { value: place.clone(), reason: *reason });
                }
            }
        }
        AliasingEffect::MutateConditionally { value } => {
            if let Some(places) = subs.get(&value.identifier.id) {
                for place in places {
                    out.push(AliasingEffect::MutateConditionally { value: place.clone() });
                }
            }
        }
        AliasingEffect::MutateTransitive { value } => {
            if let Some(places) = subs.get(&value.identifier.id) {
                for place in places {
                    out.push(AliasingEffect::MutateTransitive { value: place.clone() });
                }
            }
        }
        AliasingEffect::MutateTransitiveConditionally { value } => {
            if let Some(places) = subs.get(&value.identifier.id) {
                for place in places {
                    out.push(AliasingEffect::MutateTransitiveConditionally {
                        value: place.clone(),
                    });
                }
            }
        }
        AliasingEffect::Apply {
            receiver,
            function,
            mutates_function,
            args,
            into,
            signature,
            ..
        } => {
            let recv_places = subs.get(&receiver.identifier.id).cloned().unwrap_or_default();
            let fn_places = subs.get(&function.identifier.id).cloned().unwrap_or_default();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            let sub_args: Vec<Place> = args
                .iter()
                .flat_map(|a| subs.get(&a.identifier.id).cloned().unwrap_or_default())
                .collect();
            for rp in &recv_places {
                for fp in &fn_places {
                    for ip in &into_places {
                        out.push(AliasingEffect::Apply {
                            receiver: rp.clone(),
                            function: fp.clone(),
                            mutates_function: *mutates_function,
                            args: sub_args.clone(),
                            into: ip.clone(),
                            signature: signature.clone(),
                            loc,
                        });
                    }
                }
            }
        }
        AliasingEffect::CreateFunction { captures, function, into } => {
            let cap_places: Vec<Place> = captures
                .iter()
                .flat_map(|c| subs.get(&c.identifier.id).cloned().unwrap_or_default())
                .collect();
            let into_places = subs.get(&into.identifier.id).cloned().unwrap_or_default();
            for ip in &into_places {
                out.push(AliasingEffect::CreateFunction {
                    captures: cap_places.clone(),
                    function: function.clone(),
                    into: ip.clone(),
                });
            }
        }
        // Diagnostic effects pass through without substitution
        AliasingEffect::MutateFrozen { .. }
        | AliasingEffect::MutateGlobal { .. }
        | AliasingEffect::Impure { .. }
        | AliasingEffect::Render { .. } => {
            out.push(effect.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// Legacy signature resolution
// ---------------------------------------------------------------------------

/// Compute effects for a legacy-style function signature.
///
/// Upstream: `computeEffectsForLegacySignature()`.
fn compute_effects_for_legacy_signature(
    state: &InferenceState,
    signature: &FunctionSignature,
    lvalue: &Place,
    receiver: &Place,
    args: &[Place],
    _loc: SourceLocation,
) -> Vec<AliasingEffect> {
    let mut effects = Vec::new();

    // Upstream: `mutableOnlyIfOperandsAreMutable` optimization.
    // If the flag is set and all arguments are immutable (frozen/primitive/global),
    // treat the call as a pure read: the return is frozen and all args are just read.
    if signature.mutable_only_if_operands_are_mutable && are_arguments_immutable(state, args) {
        effects.push(AliasingEffect::Create {
            into: lvalue.clone(),
            value: ValueKind::Frozen,
            reason: ValueReason::KnownReturnSignature,
        });
        effects.push(AliasingEffect::Alias { from: receiver.clone(), into: lvalue.clone() });
        for arg in args {
            effects
                .push(AliasingEffect::ImmutableCapture { from: arg.clone(), into: lvalue.clone() });
        }
        return effects;
    }

    // Create return value
    let return_kind = signature.return_effect.into_value_kind();
    effects.push(AliasingEffect::Create {
        into: lvalue.clone(),
        value: return_kind,
        reason: ValueReason::KnownReturnSignature,
    });

    // Process callee effect
    let mut stores: Vec<Place> = Vec::new();
    let mut captures: Vec<Place> = Vec::new();

    // Callee effect
    if signature.callee_effect != Effect::Capture {
        effects.push(AliasingEffect::Alias { from: receiver.clone(), into: lvalue.clone() });
    }
    apply_param_effect(
        &mut effects,
        receiver,
        lvalue,
        signature.callee_effect,
        &mut stores,
        &mut captures,
    );

    // Per-argument effects
    for (i, arg) in args.iter().enumerate() {
        let param_effect = if i < signature.params.len() {
            signature.params[i].effect
        } else {
            // Rest/overflow params default to ConditionallyMutate
            Effect::ConditionallyMutate
        };

        apply_param_effect(&mut effects, arg, lvalue, param_effect, &mut stores, &mut captures);

        // If this param aliases to return
        if i < signature.params.len() && signature.params[i].alias_to_return {
            effects.push(AliasingEffect::Alias { from: arg.clone(), into: lvalue.clone() });
        }
    }

    // Resolve captures: if there are stores, captures go into stores; otherwise alias to return
    if !captures.is_empty() {
        if stores.is_empty() {
            for capture in &captures {
                effects.push(AliasingEffect::Alias { from: capture.clone(), into: lvalue.clone() });
            }
        } else {
            for capture in &captures {
                for store in &stores {
                    effects.push(AliasingEffect::Capture {
                        from: capture.clone(),
                        into: store.clone(),
                    });
                }
            }
        }
    }

    effects
}

/// Check if all arguments are immutable (frozen, primitive, or global).
///
/// Upstream: `areArgumentsImmutableAndNonMutating()`.
fn are_arguments_immutable(state: &InferenceState, args: &[Place]) -> bool {
    args.iter().all(|arg| {
        let kind = state.kind(arg.identifier.id).kind;
        matches!(kind, ValueKind::Frozen | ValueKind::Primitive | ValueKind::Global)
    })
}

/// Apply a single parameter effect.
fn apply_param_effect(
    effects: &mut Vec<AliasingEffect>,
    place: &Place,
    lvalue: &Place,
    effect: Effect,
    stores: &mut Vec<Place>,
    captures: &mut Vec<Place>,
) {
    match effect {
        Effect::Store => {
            effects.push(AliasingEffect::Mutate { value: place.clone(), reason: None });
            stores.push(place.clone());
        }
        Effect::Capture => {
            captures.push(place.clone());
        }
        Effect::ConditionallyMutate => {
            effects.push(AliasingEffect::MutateTransitiveConditionally { value: place.clone() });
        }
        Effect::Freeze => {
            effects.push(AliasingEffect::Freeze {
                value: place.clone(),
                reason: ValueReason::KnownReturnSignature,
            });
        }
        Effect::Mutate => {
            effects.push(AliasingEffect::MutateTransitive { value: place.clone() });
        }
        Effect::Read => {
            effects.push(AliasingEffect::ImmutableCapture {
                from: place.clone(),
                into: lvalue.clone(),
            });
        }
        Effect::Unknown | Effect::ConditionallyMutateIterator => {
            // No effect
        }
    }
}

/// Convert Effect to ValueKind for return value creation.
trait EffectToValueKind {
    fn into_value_kind(self) -> ValueKind;
}

impl EffectToValueKind for Effect {
    fn into_value_kind(self) -> ValueKind {
        match self {
            Effect::Freeze => ValueKind::Frozen,
            _ => ValueKind::Mutable,
        }
    }
}

// ---------------------------------------------------------------------------
// Terminal effects
// ---------------------------------------------------------------------------

/// Apply effects for the terminal of a block.
///
/// Upstream: terminal handling in `inferBlock()`.
fn apply_terminal_effects(
    ctx: &mut InferenceContext,
    state: &mut InferenceState,
    hir: &mut HIR,
    block_idx: usize,
) {
    let terminal = &hir.blocks[block_idx].1.terminal;

    match terminal {
        Terminal::Return { value, .. } => {
            if !ctx.is_function_expression {
                // Top-level return freezes the returned value (it's rendered as JSX)
                let freeze_effect = AliasingEffect::Freeze {
                    value: value.clone(),
                    reason: ValueReason::JsxCaptured,
                };
                let did_freeze = state.freeze(value.identifier.id, ValueReason::JsxCaptured);
                if did_freeze {
                    hir.blocks[block_idx].1.terminal = Terminal::Return {
                        value: value.clone(),
                        effects: Some(vec![freeze_effect]),
                    };
                }
            }
        }
        Terminal::MaybeThrow { handler, .. } => {
            // Upstream: alias mutable/context call results in this block to the
            // catch handler binding. This models the fact that thrown exceptions
            // from calls in a try block may be caught by the handler.
            if let Some(handler_param) = ctx.catch_handlers.get(handler).cloned() {
                let mut terminal_effects = Vec::new();

                // Find all call results in this block
                let block = &hir.blocks[block_idx].1;
                for instr in &block.instructions {
                    let is_call = matches!(
                        instr.value,
                        InstructionValue::CallExpression { .. }
                            | InstructionValue::MethodCall { .. }
                            | InstructionValue::NewExpression { .. }
                    );
                    if is_call {
                        // Alias call result to catch handler binding if mutable
                        state.append_alias(handler_param.identifier.id, instr.lvalue.identifier.id);
                        let kind = state.kind(instr.lvalue.identifier.id).kind;
                        if matches!(kind, ValueKind::Mutable | ValueKind::Context) {
                            terminal_effects.push(AliasingEffect::Alias {
                                from: instr.lvalue.clone(),
                                into: handler_param.clone(),
                            });
                        }
                    }
                }

                if !terminal_effects.is_empty() {
                    hir.blocks[block_idx].1.terminal = Terminal::MaybeThrow {
                        continuation: match &hir.blocks[block_idx].1.terminal {
                            Terminal::MaybeThrow { continuation, .. } => *continuation,
                            _ => unreachable!(),
                        },
                        handler: *handler,
                        effects: Some(terminal_effects),
                    };
                }
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// write_place_effects — final pass to annotate Place.effect
// ---------------------------------------------------------------------------

/// Write Place.effect on all operand places based on the abstract state.
///
/// This is the final pass that converts the abstract state into concrete
/// `Effect` annotations on each place in the HIR.
fn write_place_effects(
    hir: &mut HIR,
    states_by_block: &FxHashMap<BlockId, InferenceState>,
    _block_index: &FxHashMap<BlockId, usize>,
) {
    // For each block, use the final state to compute effects
    for (block_id, block) in &mut hir.blocks {
        let Some(state) = states_by_block.get(block_id) else {
            continue;
        };

        for instr in &mut block.instructions {
            // Set effect on lvalue
            let lvalue_effect = compute_place_effect(state, instr.lvalue.identifier.id);
            if lvalue_effect != Effect::Unknown {
                instr.lvalue.effect = lvalue_effect;
            }

            // Set effects on operand places
            set_operand_effects(&mut instr.value, state);
        }
    }
}

/// Compute the Effect for a place based on its abstract value kind.
fn compute_place_effect(state: &InferenceState, id: IdentifierId) -> Effect {
    if !state.is_defined(id) {
        return Effect::Unknown;
    }
    let abs_val = state.kind(id);
    match abs_val.kind {
        ValueKind::Frozen => Effect::Freeze,
        ValueKind::Primitive | ValueKind::Global => Effect::Read,
        _ => Effect::Read, // Default to Read; specific effects come from instruction.effects
    }
}

/// Set effects on operand places within an instruction value.
fn set_operand_effects(value: &mut InstructionValue, state: &InferenceState) {
    use crate::hir::types::{ArrayElement, ObjectPropertyKey};

    fn update_place(place: &mut Place, state: &InferenceState) {
        let effect = compute_place_effect(state, place.identifier.id);
        if effect != Effect::Unknown {
            place.effect = effect;
        }
    }

    match value {
        InstructionValue::LoadLocal { place }
        | InstructionValue::LoadContext { place }
        | InstructionValue::TypeCastExpression { value: place, .. }
        | InstructionValue::UnaryExpression { value: place, .. }
        | InstructionValue::PostfixUpdate { lvalue: place, .. }
        | InstructionValue::PrefixUpdate { lvalue: place, .. }
        | InstructionValue::Await { value: place }
        | InstructionValue::GetIterator { collection: place }
        | InstructionValue::NextPropertyOf { value: place } => {
            update_place(place, state);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            update_place(iterator, state);
        }
        InstructionValue::StoreLocal { value: place, .. }
        | InstructionValue::StoreContext { value: place, .. } => {
            update_place(place, state);
        }
        InstructionValue::StoreGlobal { value: place, .. } => {
            update_place(place, state);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            update_place(left, state);
            update_place(right, state);
        }
        InstructionValue::PropertyLoad { object, .. } => {
            update_place(object, state);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            update_place(object, state);
            update_place(value, state);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            update_place(object, state);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            update_place(object, state);
            update_place(property, state);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            update_place(object, state);
            update_place(property, state);
            update_place(value, state);
        }
        InstructionValue::ComputedDelete { object, property } => {
            update_place(object, state);
            update_place(property, state);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            update_place(callee, state);
            for arg in args.iter_mut() {
                update_place(arg, state);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            update_place(callee, state);
            for arg in args.iter_mut() {
                update_place(arg, state);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            update_place(receiver, state);
            for arg in args.iter_mut() {
                update_place(arg, state);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties.iter_mut() {
                if let ObjectPropertyKey::Computed(place) = &mut prop.key {
                    update_place(place, state);
                }
                update_place(&mut prop.value, state);
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements.iter_mut() {
                match elem {
                    ArrayElement::Expression(place) | ArrayElement::Spread(place) => {
                        update_place(place, state);
                    }
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            update_place(tag, state);
            for prop in props.iter_mut() {
                update_place(&mut prop.value, state);
            }
            for child in children.iter_mut() {
                update_place(child, state);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children.iter_mut() {
                update_place(child, state);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for expr in subexpressions.iter_mut() {
                update_place(expr, state);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, .. } => {
            update_place(tag, state);
        }
        InstructionValue::Destructure { value, .. } => {
            update_place(value, state);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            update_place(decl, state);
            for dep in deps.iter_mut() {
                update_place(dep, state);
            }
        }
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::DeclareLocal { .. }
        | InstructionValue::DeclareContext { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// Error message helpers
// ---------------------------------------------------------------------------

/// Generate a human-readable error reason for mutation of a frozen/global value.
fn get_write_error_reason(value: &AbstractValue) -> String {
    let reasons: Vec<&str> = value
        .reasons
        .iter()
        .map(|r| match r {
            ValueReason::ReactiveFunctionArgument => {
                "Cannot mutate a value passed as a prop or argument to a component/hook"
            }
            ValueReason::JsxCaptured => "Cannot mutate a value after it has been passed to JSX",
            ValueReason::HookCaptured => "Cannot mutate a value after it has been passed to a hook",
            ValueReason::HookReturn => {
                "Cannot mutate a value returned from a hook — it may be memoized"
            }
            _ => "Cannot mutate a frozen value",
        })
        .collect();

    if reasons.is_empty() {
        "Cannot mutate a frozen value".to_string()
    } else {
        reasons[0].to_string()
    }
}

// ---------------------------------------------------------------------------
// terminal_successors
// ---------------------------------------------------------------------------

/// PERF: Push terminal successors into an existing buffer to avoid per-call Vec allocation.
fn terminal_successors_into(terminal: &Terminal, buf: &mut Vec<BlockId>) {
    match terminal {
        Terminal::Goto { block } => buf.push(*block),
        Terminal::If { consequent, alternate, fallthrough, .. } => {
            buf.extend_from_slice(&[*consequent, *alternate, *fallthrough]);
        }
        Terminal::Branch { consequent, alternate, .. } => {
            buf.extend_from_slice(&[*consequent, *alternate]);
        }
        Terminal::Switch { cases, fallthrough, .. } => {
            buf.extend(cases.iter().map(|c| c.block));
            buf.push(*fallthrough);
        }
        Terminal::Return { .. } | Terminal::Throw { .. } | Terminal::Unreachable => {}
        Terminal::For { init, test, update, body, fallthrough } => {
            buf.extend_from_slice(&[*init, *test, *body, *fallthrough]);
            if let Some(u) = update {
                buf.push(*u);
            }
        }
        Terminal::ForOf { init, test, body, fallthrough }
        | Terminal::ForIn { init, test, body, fallthrough } => {
            buf.extend_from_slice(&[*init, *test, *body, *fallthrough]);
        }
        Terminal::DoWhile { body, test, fallthrough } => {
            buf.extend_from_slice(&[*body, *test, *fallthrough]);
        }
        Terminal::While { test, body, fallthrough } => {
            buf.extend_from_slice(&[*test, *body, *fallthrough]);
        }
        Terminal::Logical { left, right, fallthrough, .. } => {
            buf.extend_from_slice(&[*left, *right, *fallthrough]);
        }
        Terminal::Ternary { consequent, alternate, fallthrough, .. } => {
            buf.extend_from_slice(&[*consequent, *alternate, *fallthrough]);
        }
        Terminal::Optional { consequent, fallthrough, .. } => {
            buf.extend_from_slice(&[*consequent, *fallthrough]);
        }
        Terminal::Sequence { blocks, fallthrough } => {
            buf.extend_from_slice(blocks);
            buf.push(*fallthrough);
        }
        Terminal::Label { block, fallthrough, .. } => {
            buf.extend_from_slice(&[*block, *fallthrough]);
        }
        Terminal::MaybeThrow { continuation, handler, .. } => {
            buf.extend_from_slice(&[*continuation, *handler]);
        }
        Terminal::Try { block, handler, fallthrough } => {
            buf.extend_from_slice(&[*block, *handler, *fallthrough]);
        }
        Terminal::Scope { block, fallthrough, .. }
        | Terminal::PrunedScope { block, fallthrough, .. } => {
            buf.extend_from_slice(&[*block, *fallthrough]);
        }
    }
}
