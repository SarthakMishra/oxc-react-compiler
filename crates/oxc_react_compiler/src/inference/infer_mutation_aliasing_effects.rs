#![allow(dead_code)]

use rustc_hash::FxHashMap;

use crate::hir::types::{
    AliasingEffect, Effect, FunctionSignature, HIR, IdentifierId, Place, ValueKind, ValueReason,
};

use super::aliasing_effects::compute_instruction_effects;

/// Abstract value representing a heap object in the abstract interpretation.
#[derive(Debug, Clone)]
struct AbstractValue {
    /// What kind of value this is (primitive, frozen, mutable, etc.)
    kind: ValueKind,
    /// Whether this value has been frozen (e.g., passed to a JSX prop).
    frozen: bool,
    /// The freeze reason, if frozen.
    freeze_reason: Option<ValueReason>,
    /// Set of IdentifierIds that alias this same abstract value.
    aliases: Vec<IdentifierId>,
    /// Set of IdentifierIds captured (indirectly referenced) by this value.
    captures: Vec<IdentifierId>,
    /// Whether this value has been mutated.
    mutated: bool,
    /// Whether this value has been conditionally mutated.
    conditionally_mutated: bool,
}

impl AbstractValue {
    fn new(kind: ValueKind) -> Self {
        Self {
            kind,
            frozen: matches!(kind, ValueKind::Frozen | ValueKind::Primitive | ValueKind::Global),
            freeze_reason: None,
            aliases: Vec::new(),
            captures: Vec::new(),
            mutated: false,
            conditionally_mutated: false,
        }
    }
}

/// Lattice join for value kinds. Returns the more pessimistic (less constrained) kind.
/// Used during fixpoint iteration when the same identifier is created with different kinds.
///
/// Lattice order (bottom to top): Primitive < Global < Frozen < MaybeFrozen < Mutable
///
/// DIVERGENCE: `ValueKind::Context` is treated as equivalent to `Mutable` in the lattice.
/// Upstream tracks context variables separately, but in our model Context values flow
/// through the same lattice and merge to Mutable when joined with any non-Context kind.
fn merge_value_kinds(a: ValueKind, b: ValueKind) -> ValueKind {
    if a == b {
        return a;
    }
    use ValueKind::{Frozen, Global, MaybeFrozen, Mutable, Primitive};
    match (a, b) {
        (Primitive, Global) | (Global, Primitive) => Global,
        (Primitive | Global, Frozen) | (Frozen, Primitive | Global) => Frozen,
        (Primitive | Global | Frozen, MaybeFrozen) | (MaybeFrozen, Primitive | Global | Frozen) => {
            MaybeFrozen
        }
        // Mutable and Context are at the top of the lattice
        _ => Mutable,
    }
}

/// The abstract heap: maps each identifier to its abstract value.
struct AbstractHeap {
    /// Maps IdentifierId -> index into `values`.
    id_to_value: FxHashMap<IdentifierId, usize>,
    /// All abstract values.
    values: Vec<AbstractValue>,
}

impl AbstractHeap {
    fn new() -> Self {
        Self { id_to_value: FxHashMap::default(), values: Vec::new() }
    }

    /// Allocate a new abstract value and associate it with the given identifier.
    /// Returns `true` if this is a new allocation (id not previously tracked),
    /// or if the kind was widened via lattice join on re-creation.
    fn create(&mut self, id: IdentifierId, kind: ValueKind) -> bool {
        if let Some(&idx) = self.id_to_value.get(&id) {
            // Already exists — update kind if it changed (lattice widening).
            let old_kind = self.values[idx].kind;
            let new_kind = merge_value_kinds(old_kind, kind);
            if new_kind != old_kind {
                self.values[idx].kind = new_kind;
                return true;
            }
            return false;
        }
        let idx = self.values.len();
        self.values.push(AbstractValue::new(kind));
        self.values[idx].aliases.push(id);
        self.id_to_value.insert(id, idx);
        true
    }

    /// Create a new abstract value derived from an existing one.
    /// Returns `true` if the heap changed.
    fn create_from(&mut self, from: IdentifierId, into: IdentifierId) -> bool {
        let kind = self
            .id_to_value
            .get(&from)
            .and_then(|&idx| self.values.get(idx))
            .map_or(ValueKind::Mutable, |v| v.kind);
        self.create(into, kind)
    }

    /// Make `into` an alias of `from` (they share the same abstract value).
    /// Returns `true` if the heap changed.
    /// Make `into` an alias of `from` (they share the same abstract value).
    /// Returns `true` if the heap changed.
    fn alias(&mut self, from: IdentifierId, into: IdentifierId) -> bool {
        if let Some(&from_idx) = self.id_to_value.get(&from) {
            if let Some(&existing_idx) = self.id_to_value.get(&into)
                && existing_idx == from_idx
            {
                return false; // Already aliased
            }
            self.id_to_value.insert(into, from_idx);
            if !self.values[from_idx].aliases.contains(&into) {
                self.values[from_idx].aliases.push(into);
            }
            true
        } else {
            // Source not tracked yet; create a fresh value for the target.
            self.create(into, ValueKind::Mutable)
        }
    }

    /// Record that `into` captures a reference to `from`.
    /// Returns `true` if the heap changed.
    fn capture(&mut self, from: IdentifierId, into: IdentifierId) -> bool {
        let mut changed = false;
        if !self.id_to_value.contains_key(&into) {
            self.create(into, ValueKind::Mutable);
            changed = true;
        }
        if let Some(&into_idx) = self.id_to_value.get(&into)
            && !self.values[into_idx].captures.contains(&from)
        {
            self.values[into_idx].captures.push(from);
            changed = true;
        }
        changed
    }

    /// Mark a value as mutated. Returns `true` if this is a new mutation.
    fn mutate(&mut self, id: IdentifierId) -> bool {
        if let Some(&idx) = self.id_to_value.get(&id)
            && !self.values[idx].mutated
        {
            self.values[idx].mutated = true;
            return true;
        }
        false
    }

    /// Mark a value as conditionally mutated. Returns `true` if changed.
    fn mutate_conditionally(&mut self, id: IdentifierId) -> bool {
        if let Some(&idx) = self.id_to_value.get(&id)
            && !self.values[idx].conditionally_mutated
        {
            self.values[idx].conditionally_mutated = true;
            return true;
        }
        false
    }

    /// Transitively mutate: mutate the value and everything it captures.
    /// Returns `true` if any mutation was new.
    fn mutate_transitive(&mut self, id: IdentifierId) -> bool {
        let mut changed = self.mutate(id);
        let captured: Vec<IdentifierId> = self
            .id_to_value
            .get(&id)
            .and_then(|&idx| self.values.get(idx))
            .map(|v| v.captures.clone())
            .unwrap_or_default();
        for cap_id in captured {
            changed |= self.mutate(cap_id);
        }
        changed
    }

    /// Freeze a value. Returns `true` if this is a new freeze.
    fn freeze(&mut self, id: IdentifierId, reason: ValueReason) -> bool {
        if let Some(&idx) = self.id_to_value.get(&id)
            && !self.values[idx].frozen
        {
            self.values[idx].frozen = true;
            self.values[idx].freeze_reason = Some(reason);
            return true;
        }
        false
    }

    /// Check if a value is frozen.
    fn is_frozen(&self, id: IdentifierId) -> bool {
        self.id_to_value.get(&id).and_then(|&idx| self.values.get(idx)).is_some_and(|v| v.frozen)
    }

    /// Get the value kind for an identifier, defaulting to Mutable if unknown.
    fn value_kind(&self, id: IdentifierId) -> ValueKind {
        self.id_to_value.get(&id).and_then(|&idx| self.values.get(idx)).map_or(
            ValueKind::Mutable,
            |v| {
                if v.frozen { ValueKind::Frozen } else { v.kind }
            },
        )
    }

    /// Compute the effect for a place based on heap state.
    fn compute_effect(&self, id: IdentifierId) -> Effect {
        let Some(&idx) = self.id_to_value.get(&id) else {
            return Effect::Unknown;
        };
        let value = &self.values[idx];

        if value.frozen {
            return Effect::Freeze;
        }
        if value.mutated {
            return Effect::Mutate;
        }
        if value.conditionally_mutated {
            return Effect::ConditionallyMutate;
        }
        if !value.captures.is_empty() {
            return Effect::Capture;
        }
        Effect::Read
    }
}

/// Infer mutation and aliasing effects for all instructions.
///
/// This is the most computationally intensive pass in the compiler.
///
/// Algorithm (interleaved fixpoint):
/// 1. Walk all instructions in program order. For each instruction:
///    a. Compute raw effects from instruction syntax
///    b. Immediately refine effects using current heap state (value kinds)
///    c. Apply refined effects to the abstract heap
///    d. Store refined effects on the instruction
/// 2. Propagate mutations transitively through capture chains
/// 3. Repeat steps 1-2 until the heap stabilizes (no new information)
/// 4. Write final effects back to all places
///
/// This interleaved approach mirrors upstream's `applyEffect()` pattern where
/// effect computation and heap building happen together, allowing later
/// instructions to see value kinds established by earlier ones.
#[expect(clippy::implicit_hasher)]
pub fn infer_mutation_aliasing_effects(
    hir: &mut HIR,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
) {
    infer_mutation_aliasing_effects_inner(hir, fn_signatures, &[]);
}

/// Inner implementation that accepts optional param_names for pre-freezing.
///
/// When compiling a component function, `param_names` contains parameter names
/// (e.g., "props") that should be treated as frozen in the abstract heap.
/// This prevents false-positive conditional mutation effects on parameters
/// from causing bail-outs in the frozen-mutation validator.
#[expect(clippy::implicit_hasher)]
pub fn infer_mutation_aliasing_effects_with_params(
    hir: &mut HIR,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    param_names: &[String],
) {
    infer_mutation_aliasing_effects_inner(hir, fn_signatures, param_names);
}

fn infer_mutation_aliasing_effects_inner(
    hir: &mut HIR,
    fn_signatures: &FxHashMap<IdentifierId, FunctionSignature>,
    param_names: &[String],
) {
    const MAX_ITERATIONS: usize = 100;
    let mut heap = AbstractHeap::new();
    let mut iteration = 0;
    let mut global_changed = true;

    // Pre-freeze function parameters in the heap. Component props and hook
    // arguments are frozen — the function receives them immutably. This ensures
    // that refine_effects drops MutateConditionally/MutateTransitiveConditionally
    // effects on parameters (from conservative Apply fallback), preventing
    // false positives in the frozen-mutation validator.
    if !param_names.is_empty() {
        pre_freeze_params(hir, &mut heap, param_names);
    }

    // Outer fixpoint loop: re-walk all instructions until heap stabilizes.
    // On each iteration, effects are recomputed using the current heap state,
    // which may have been enriched by the previous iteration's refinements.
    while global_changed && iteration < MAX_ITERATIONS {
        global_changed = false;
        iteration += 1;

        // Phase 1+2+3.5 (interleaved): compute → refine → apply to heap
        for (_, block) in &mut hir.blocks {
            for instr in &mut block.instructions {
                // Compute raw effects from instruction syntax
                let raw_effects = compute_instruction_effects(
                    &instr.value,
                    &instr.lvalue,
                    instr.loc,
                    fn_signatures,
                );

                // Immediately refine using current heap state
                let refined = refine_effects(&raw_effects, &heap);

                // Apply refined effects to heap, tracking changes
                for effect in &refined {
                    if process_effect_for_heap(&mut heap, effect) {
                        global_changed = true;
                    }
                }

                // Store refined effects on instruction
                instr.effects = Some(refined);
            }
        }

        // Phase 3: Propagate unconditional mutations through capture chains (fixpoint).
        // Only unconditional mutations propagate here — conditional mutations are already
        // propagated inline by MutateTransitiveConditionally in process_effect_for_heap.
        let mut prop_changed = true;
        let mut prop_iterations = 0;
        while prop_changed && prop_iterations < MAX_ITERATIONS {
            prop_changed = false;
            prop_iterations += 1;

            let mut to_mutate: Vec<usize> = Vec::new();

            for value_idx in 0..heap.values.len() {
                if !heap.values[value_idx].mutated && !heap.values[value_idx].conditionally_mutated
                {
                    continue;
                }

                let is_mutated = heap.values[value_idx].mutated;
                let captured: Vec<IdentifierId> = heap.values[value_idx].captures.clone();

                for cap_id in captured {
                    if let Some(&cap_idx) = heap.id_to_value.get(&cap_id)
                        && !heap.values[cap_idx].mutated
                        && is_mutated
                    {
                        to_mutate.push(cap_idx);
                    }
                }
            }

            for idx in to_mutate {
                if !heap.values[idx].mutated {
                    heap.values[idx].mutated = true;
                    prop_changed = true;
                    global_changed = true;
                }
            }
        }
    }

    // Phase 4: Write effects back to all places.
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            // Set effect on the lvalue.
            let lvalue_effect = heap.compute_effect(instr.lvalue.identifier.id);
            if lvalue_effect != Effect::Unknown {
                instr.lvalue.effect = lvalue_effect;
            }

            // Set effects on operand places within the instruction value.
            set_operand_effects(&mut instr.value, &heap);
        }
    }
}

/// Pre-freeze function parameters in the abstract heap.
///
/// Component props and hook arguments should be treated as frozen values.
/// This seeds the heap so that `refine_effects` can properly drop speculative
/// mutation effects (MutateConditionally, MutateTransitiveConditionally) on
/// parameters. Without this, the conservative Apply fallback emits conditional
/// mutations on all args, which falsely triggers the frozen-mutation validator.
///
/// DIVERGENCE: Our HIR creates fresh IdentifierIds per Place reference, so we
/// must walk ALL places in ALL instructions to find every ID that refers to a
/// parameter by name. Upstream's pointer-identity model avoids this issue.
fn pre_freeze_params(hir: &HIR, heap: &mut AbstractHeap, param_names: &[String]) {
    use crate::hir::types::{ArrayElement, InstructionValue, ObjectPropertyKey};

    let freeze_if_param = |place: &Place, heap: &mut AbstractHeap| {
        if let Some(name) = &place.identifier.name
            && param_names.iter().any(|p| p == name)
        {
            heap.create(place.identifier.id, ValueKind::Frozen);
            heap.freeze(place.identifier.id, ValueReason::ReactiveFunctionArgument);
        }
    };

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            // Freeze lvalue if it's a param
            freeze_if_param(&instr.lvalue, heap);

            // Walk all operand places
            match &instr.value {
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
                    freeze_if_param(place, heap);
                }
                InstructionValue::StoreLocal { lvalue, value, .. }
                | InstructionValue::StoreContext { lvalue, value } => {
                    freeze_if_param(lvalue, heap);
                    freeze_if_param(value, heap);
                }
                InstructionValue::DeclareLocal { lvalue, .. }
                | InstructionValue::DeclareContext { lvalue } => {
                    freeze_if_param(lvalue, heap);
                }
                InstructionValue::CallExpression { callee, args, .. }
                | InstructionValue::NewExpression { callee, args } => {
                    freeze_if_param(callee, heap);
                    for arg in args {
                        freeze_if_param(arg, heap);
                    }
                }
                InstructionValue::MethodCall { receiver, args, .. } => {
                    freeze_if_param(receiver, heap);
                    for arg in args {
                        freeze_if_param(arg, heap);
                    }
                }
                InstructionValue::BinaryExpression { left, right, .. } => {
                    freeze_if_param(left, heap);
                    freeze_if_param(right, heap);
                }
                InstructionValue::PropertyLoad { object, .. }
                | InstructionValue::PropertyDelete { object, .. } => {
                    freeze_if_param(object, heap);
                }
                InstructionValue::PropertyStore { object, value, .. } => {
                    freeze_if_param(object, heap);
                    freeze_if_param(value, heap);
                }
                InstructionValue::ComputedLoad { object, property, .. }
                | InstructionValue::ComputedDelete { object, property } => {
                    freeze_if_param(object, heap);
                    freeze_if_param(property, heap);
                }
                InstructionValue::ComputedStore { object, property, value } => {
                    freeze_if_param(object, heap);
                    freeze_if_param(property, heap);
                    freeze_if_param(value, heap);
                }
                InstructionValue::ObjectExpression { properties } => {
                    for prop in properties {
                        if let ObjectPropertyKey::Computed(p) = &prop.key {
                            freeze_if_param(p, heap);
                        }
                        freeze_if_param(&prop.value, heap);
                    }
                }
                InstructionValue::ArrayExpression { elements } => {
                    for el in elements {
                        match el {
                            ArrayElement::Expression(p) | ArrayElement::Spread(p) => {
                                freeze_if_param(p, heap);
                            }
                            ArrayElement::Hole => {}
                        }
                    }
                }
                InstructionValue::JsxExpression { tag, props, children } => {
                    freeze_if_param(tag, heap);
                    for prop in props {
                        freeze_if_param(&prop.value, heap);
                    }
                    for child in children {
                        freeze_if_param(child, heap);
                    }
                }
                InstructionValue::JsxFragment { children } => {
                    for child in children {
                        freeze_if_param(child, heap);
                    }
                }
                InstructionValue::Destructure { value, .. } => {
                    freeze_if_param(value, heap);
                }
                InstructionValue::IteratorNext { iterator, .. } => {
                    freeze_if_param(iterator, heap);
                }
                InstructionValue::TaggedTemplateExpression { tag, value: tagged_value, .. } => {
                    freeze_if_param(tag, heap);
                    for expr in &tagged_value.subexpressions {
                        freeze_if_param(expr, heap);
                    }
                }
                InstructionValue::TemplateLiteral { subexpressions, .. } => {
                    for expr in subexpressions {
                        freeze_if_param(expr, heap);
                    }
                }
                InstructionValue::FinishMemoize { decl, deps, .. } => {
                    freeze_if_param(decl, heap);
                    for dep in deps {
                        freeze_if_param(dep, heap);
                    }
                }
                InstructionValue::FunctionExpression { lowered_func, .. } => {
                    for ctx_place in &lowered_func.context {
                        freeze_if_param(ctx_place, heap);
                    }
                }
                // No operand places:
                InstructionValue::Primitive { .. }
                | InstructionValue::JSXText { .. }
                | InstructionValue::LoadGlobal { .. }
                | InstructionValue::RegExpLiteral { .. }
                | InstructionValue::ObjectMethod { .. }
                | InstructionValue::StartMemoize { .. }
                | InstructionValue::UnsupportedNode { .. } => {}
            }
        }
    }
}

/// Process a single aliasing effect to build the heap model.
/// Returns `true` if the heap was modified (new information added).
fn process_effect_for_heap(heap: &mut AbstractHeap, effect: &AliasingEffect) -> bool {
    match effect {
        AliasingEffect::Create { into, value, .. } => heap.create(into.identifier.id, *value),
        AliasingEffect::CreateFrom { from, into } => {
            heap.create_from(from.identifier.id, into.identifier.id)
        }
        AliasingEffect::CreateFunction { captures, function: _, into } => {
            let mut changed = heap.create(into.identifier.id, ValueKind::Mutable);
            for cap in captures {
                changed |= heap.capture(cap.identifier.id, into.identifier.id);
            }
            changed
        }
        AliasingEffect::Apply { .. } => {
            // Apply effects are always resolved by refine_effects() before reaching
            // process_effect_for_heap() in the interleaved architecture. If we get here,
            // it means refine_effects() missed an Apply — which shouldn't happen.
            // Skip gracefully rather than panicking; the Apply was already handled
            // by refine_effects() producing concrete Create/Mutate/Capture effects.
            false
        }
        AliasingEffect::Assign { from, into } | AliasingEffect::Alias { from, into } => {
            heap.alias(from.identifier.id, into.identifier.id)
        }
        AliasingEffect::MaybeAlias { from, into } => {
            heap.alias(from.identifier.id, into.identifier.id)
        }
        AliasingEffect::Capture { from, into } => {
            heap.capture(from.identifier.id, into.identifier.id)
        }
        AliasingEffect::ImmutableCapture { from, into } => {
            let mut changed = heap.capture(from.identifier.id, into.identifier.id);
            changed |= heap.freeze(from.identifier.id, ValueReason::JsxCaptured);
            changed
        }
        AliasingEffect::Mutate { value, .. } => heap.mutate(value.identifier.id),
        AliasingEffect::MutateConditionally { value } => {
            heap.mutate_conditionally(value.identifier.id)
        }
        AliasingEffect::MutateTransitive { value } => heap.mutate_transitive(value.identifier.id),
        AliasingEffect::MutateTransitiveConditionally { value } => {
            let mut changed = heap.mutate_conditionally(value.identifier.id);
            let captured: Vec<IdentifierId> = heap
                .id_to_value
                .get(&value.identifier.id)
                .and_then(|&idx| heap.values.get(idx))
                .map(|v| v.captures.clone())
                .unwrap_or_default();
            for cap_id in captured {
                changed |= heap.mutate_conditionally(cap_id);
            }
            changed
        }
        AliasingEffect::Freeze { value, reason } => heap.freeze(value.identifier.id, *reason),
        AliasingEffect::MutateFrozen { .. }
        | AliasingEffect::MutateGlobal { .. }
        | AliasingEffect::Impure { .. }
        | AliasingEffect::Render { .. } => {
            // These are diagnostic effects -- they don't modify the heap model.
            false
        }
    }
}

/// Refine raw effects based on the abstract heap's value kind analysis.
///
/// Upstream: `applyEffect()` in `InferMutationAliasingEffects.ts`
///
/// Key refinements:
/// - `Apply` → resolved into `Create` for return + `MutateTransitiveConditionally` for each arg
/// - `CreateFrom` where source is Primitive/Global → `Create(Primitive)` (no alias edge)
/// - `Capture` where source is Primitive/Global → dropped (no capture edge)
/// - `Assign` where source is Primitive/Global → `Create(Primitive)` (no alias edge)
/// - `MutateConditionally` where target is Primitive/Global/Frozen → dropped
/// - `Mutate` where target is Primitive/Global → dropped
fn refine_effects(effects: &[AliasingEffect], heap: &AbstractHeap) -> Vec<AliasingEffect> {
    let mut refined = Vec::with_capacity(effects.len());

    for effect in effects {
        match effect {
            AliasingEffect::Apply { receiver, args, into, signature, .. } => {
                // Resolve Apply into concrete effects
                if let Some(sig) = signature {
                    // Signature-aware resolution: use per-parameter effects
                    refined.push(AliasingEffect::Create {
                        into: into.clone(),
                        value: ValueKind::Mutable,
                        reason: crate::hir::types::ValueReason::Other,
                    });
                    for (arg, param_effect) in args.iter().zip(sig.params.iter()) {
                        let kind = heap.value_kind(arg.identifier.id);
                        if matches!(kind, ValueKind::Primitive | ValueKind::Global) {
                            continue; // Skip effects on known primitives/globals
                        }
                        match param_effect.effect {
                            Effect::Mutate => {
                                refined.push(AliasingEffect::Mutate {
                                    value: arg.clone(),
                                    reason: None,
                                });
                            }
                            Effect::ConditionallyMutate => {
                                refined.push(AliasingEffect::MutateConditionally {
                                    value: arg.clone(),
                                });
                            }
                            Effect::Capture => {
                                refined.push(AliasingEffect::Capture {
                                    from: arg.clone(),
                                    into: into.clone(),
                                });
                            }
                            Effect::Freeze => {
                                refined.push(AliasingEffect::Freeze {
                                    value: arg.clone(),
                                    reason: ValueReason::KnownReturnSignature,
                                });
                            }
                            _ => {} // Read, Store, Unknown → no effect on arg
                        }
                        if param_effect.alias_to_return {
                            refined.push(AliasingEffect::Alias {
                                from: arg.clone(),
                                into: into.clone(),
                            });
                        }
                    }
                } else {
                    // No signature: conservative fallback
                    // Create a mutable value for the return
                    // MutateTransitiveConditionally each arg
                    // MaybeAlias each arg to return value
                    // Cross-arg Capture (each arg may be stored into each other arg)
                    refined.push(AliasingEffect::Create {
                        into: into.clone(),
                        value: ValueKind::Mutable,
                        reason: crate::hir::types::ValueReason::Other,
                    });

                    // Build the full operand set: receiver (if non-trivial) + args
                    let mut operands: Vec<&Place> = Vec::new();
                    let receiver_kind = heap.value_kind(receiver.identifier.id);
                    if !matches!(receiver_kind, ValueKind::Primitive | ValueKind::Global) {
                        operands.push(receiver);
                    }
                    for arg in args {
                        let kind = heap.value_kind(arg.identifier.id);
                        if !matches!(kind, ValueKind::Primitive | ValueKind::Global) {
                            operands.push(arg);
                        }
                    }

                    // MutateTransitiveConditionally for each arg (not receiver)
                    for arg in args {
                        let kind = heap.value_kind(arg.identifier.id);
                        if !matches!(kind, ValueKind::Primitive | ValueKind::Global) {
                            refined.push(AliasingEffect::MutateTransitiveConditionally {
                                value: arg.clone(),
                            });
                        }
                    }

                    // MaybeAlias: each operand may alias the return value
                    for operand in &operands {
                        refined.push(AliasingEffect::MaybeAlias {
                            from: (*operand).clone(),
                            into: into.clone(),
                        });
                    }

                    // Cross-arg Capture: each operand may be captured into each other operand
                    for (i, operand) in operands.iter().enumerate() {
                        for (j, other) in operands.iter().enumerate() {
                            if i == j {
                                continue;
                            }
                            refined.push(AliasingEffect::Capture {
                                from: (*operand).clone(),
                                into: (*other).clone(),
                            });
                        }
                    }
                }
            }

            AliasingEffect::CreateFrom { from, into } => {
                let kind = heap.value_kind(from.identifier.id);
                match kind {
                    ValueKind::Primitive | ValueKind::Global => {
                        // Source is primitive/global → create a primitive, no alias
                        refined.push(AliasingEffect::Create {
                            into: into.clone(),
                            value: ValueKind::Primitive,
                            reason: crate::hir::types::ValueReason::KnownValue,
                        });
                    }
                    ValueKind::Frozen | ValueKind::MaybeFrozen => {
                        // Source is frozen → create frozen + immutable capture
                        refined.push(AliasingEffect::Create {
                            into: into.clone(),
                            value: ValueKind::Frozen,
                            reason: crate::hir::types::ValueReason::KnownValue,
                        });
                        refined.push(AliasingEffect::ImmutableCapture {
                            from: from.clone(),
                            into: into.clone(),
                        });
                    }
                    ValueKind::Mutable | ValueKind::Context => {
                        // Keep as-is
                        refined.push(effect.clone());
                    }
                }
            }

            AliasingEffect::Assign { from, into } => {
                let kind = heap.value_kind(from.identifier.id);
                match kind {
                    ValueKind::Primitive | ValueKind::Global => {
                        // Assigning from primitive → target gets a primitive, no alias edge
                        refined.push(AliasingEffect::Create {
                            into: into.clone(),
                            value: ValueKind::Primitive,
                            reason: crate::hir::types::ValueReason::KnownValue,
                        });
                    }
                    ValueKind::Frozen | ValueKind::MaybeFrozen => {
                        // Assigning from frozen → create frozen target + immutable capture
                        refined.push(AliasingEffect::Create {
                            into: into.clone(),
                            value: ValueKind::Frozen,
                            reason: crate::hir::types::ValueReason::KnownValue,
                        });
                        refined.push(AliasingEffect::ImmutableCapture {
                            from: from.clone(),
                            into: into.clone(),
                        });
                    }
                    ValueKind::Mutable | ValueKind::Context => {
                        refined.push(effect.clone());
                    }
                }
            }

            AliasingEffect::Capture { from, into } => {
                let from_kind = heap.value_kind(from.identifier.id);
                match from_kind {
                    ValueKind::Primitive | ValueKind::Global => {
                        // Capturing a primitive/global is a no-op
                    }
                    ValueKind::Frozen | ValueKind::MaybeFrozen => {
                        // Capturing a frozen value → immutable capture
                        refined.push(AliasingEffect::ImmutableCapture {
                            from: from.clone(),
                            into: into.clone(),
                        });
                    }
                    ValueKind::Context => {
                        // Upstream: when `from` is Context, upgrade Capture to
                        // MaybeAlias if `into` is Mutable/MaybeFrozen/Context.
                        // Context values may reference the same underlying state,
                        // so capturing into a mutable container creates an alias.
                        let into_kind = heap.value_kind(into.identifier.id);
                        if matches!(
                            into_kind,
                            ValueKind::Mutable | ValueKind::MaybeFrozen | ValueKind::Context
                        ) {
                            refined.push(AliasingEffect::MaybeAlias {
                                from: from.clone(),
                                into: into.clone(),
                            });
                        } else {
                            refined.push(effect.clone());
                        }
                    }
                    ValueKind::Mutable => {
                        refined.push(effect.clone());
                    }
                }
            }

            AliasingEffect::ImmutableCapture { from, .. } => {
                let kind = heap.value_kind(from.identifier.id);
                if matches!(kind, ValueKind::Primitive | ValueKind::Global) {
                    // Immutable capture of primitive/global is a no-op
                } else {
                    refined.push(effect.clone());
                }
            }

            AliasingEffect::MutateConditionally { value } => {
                let kind = heap.value_kind(value.identifier.id);
                match kind {
                    ValueKind::Primitive | ValueKind::Global | ValueKind::Frozen => {
                        // Conditionally mutating a known-immutable → drop
                    }
                    ValueKind::MaybeFrozen => {
                        // MaybeFrozen being conditionally mutated → MutateFrozen error
                        refined.push(AliasingEffect::MutateFrozen {
                            place: value.clone(),
                            error: "Cannot mutate a value that may be frozen".to_string(),
                        });
                    }
                    ValueKind::Mutable | ValueKind::Context => {
                        refined.push(effect.clone());
                    }
                }
            }

            AliasingEffect::MutateTransitiveConditionally { value } => {
                let kind = heap.value_kind(value.identifier.id);
                match kind {
                    ValueKind::Primitive | ValueKind::Global | ValueKind::Frozen => {
                        // Drop
                    }
                    ValueKind::MaybeFrozen => {
                        refined.push(AliasingEffect::MutateFrozen {
                            place: value.clone(),
                            error: "Cannot mutate a value that may be frozen".to_string(),
                        });
                    }
                    ValueKind::Mutable | ValueKind::Context => {
                        refined.push(effect.clone());
                    }
                }
            }

            AliasingEffect::Mutate { value, .. } | AliasingEffect::MutateTransitive { value } => {
                let kind = heap.value_kind(value.identifier.id);
                match kind {
                    ValueKind::Primitive | ValueKind::Global => {
                        // Mutating a primitive/global → drop
                    }
                    ValueKind::Frozen | ValueKind::MaybeFrozen => {
                        // Mutating a frozen value → MutateFrozen error
                        refined.push(AliasingEffect::MutateFrozen {
                            place: value.clone(),
                            error: "Cannot mutate a frozen value".to_string(),
                        });
                    }
                    ValueKind::Mutable | ValueKind::Context => {
                        refined.push(effect.clone());
                    }
                }
            }

            // Pass through all other effects unchanged
            AliasingEffect::Create { .. }
            | AliasingEffect::CreateFunction { .. }
            | AliasingEffect::Alias { .. }
            | AliasingEffect::MaybeAlias { .. }
            | AliasingEffect::Freeze { .. }
            | AliasingEffect::MutateFrozen { .. }
            | AliasingEffect::MutateGlobal { .. }
            | AliasingEffect::Impure { .. }
            | AliasingEffect::Render { .. } => {
                refined.push(effect.clone());
            }
        }
    }

    refined
}

/// Set effects on operand places within an instruction value.
/// This walks the InstructionValue and updates the `effect` field on each Place operand.
fn set_operand_effects(value: &mut crate::hir::types::InstructionValue, heap: &AbstractHeap) {
    use crate::hir::types::{ArrayElement, InstructionValue, ObjectPropertyKey};

    // Helper: update a place's effect from the heap.
    fn update_place(place: &mut Place, heap: &AbstractHeap) {
        let effect = heap.compute_effect(place.identifier.id);
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
            update_place(place, heap);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            update_place(iterator, heap);
        }
        InstructionValue::StoreLocal { value: place, .. }
        | InstructionValue::StoreContext { value: place, .. } => {
            update_place(place, heap);
        }
        InstructionValue::StoreGlobal { value: place, .. } => {
            update_place(place, heap);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            update_place(left, heap);
            update_place(right, heap);
        }
        InstructionValue::PropertyLoad { object, .. } => {
            update_place(object, heap);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            update_place(object, heap);
            update_place(value, heap);
        }
        InstructionValue::PropertyDelete { object, .. } => {
            update_place(object, heap);
        }
        InstructionValue::ComputedLoad { object, property, .. } => {
            update_place(object, heap);
            update_place(property, heap);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            update_place(object, heap);
            update_place(property, heap);
            update_place(value, heap);
        }
        InstructionValue::ComputedDelete { object, property } => {
            update_place(object, heap);
            update_place(property, heap);
        }
        InstructionValue::CallExpression { callee, args, .. } => {
            update_place(callee, heap);
            for arg in args.iter_mut() {
                update_place(arg, heap);
            }
        }
        InstructionValue::NewExpression { callee, args } => {
            update_place(callee, heap);
            for arg in args.iter_mut() {
                update_place(arg, heap);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            update_place(receiver, heap);
            for arg in args.iter_mut() {
                update_place(arg, heap);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties.iter_mut() {
                if let ObjectPropertyKey::Computed(place) = &mut prop.key {
                    update_place(place, heap);
                }
                update_place(&mut prop.value, heap);
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements.iter_mut() {
                match elem {
                    ArrayElement::Expression(place) | ArrayElement::Spread(place) => {
                        update_place(place, heap);
                    }
                    ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            update_place(tag, heap);
            for prop in props.iter_mut() {
                update_place(&mut prop.value, heap);
            }
            for child in children.iter_mut() {
                update_place(child, heap);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children.iter_mut() {
                update_place(child, heap);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for expr in subexpressions.iter_mut() {
                update_place(expr, heap);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, .. } => {
            update_place(tag, heap);
        }
        InstructionValue::Destructure { value, .. } => {
            update_place(value, heap);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            update_place(decl, heap);
            for dep in deps.iter_mut() {
                update_place(dep, heap);
            }
        }
        // These don't have operand places to update:
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
