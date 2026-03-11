#![allow(dead_code)]

use rustc_hash::FxHashMap;

use crate::hir::types::{
    AliasingEffect, Effect, FreezeReason, HIR, IdentifierId, Place, ValueKind,
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
    freeze_reason: Option<FreezeReason>,
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
    fn create(&mut self, id: IdentifierId, kind: ValueKind) {
        let idx = self.values.len();
        self.values.push(AbstractValue::new(kind));
        self.values[idx].aliases.push(id);
        self.id_to_value.insert(id, idx);
    }

    /// Create a new abstract value derived from an existing one.
    fn create_from(&mut self, from: IdentifierId, into: IdentifierId) {
        let kind = self
            .id_to_value
            .get(&from)
            .and_then(|&idx| self.values.get(idx))
            .map(|v| v.kind)
            .unwrap_or(ValueKind::Mutable);
        self.create(into, kind);
    }

    /// Make `into` an alias of `from` (they share the same abstract value).
    fn alias(&mut self, from: IdentifierId, into: IdentifierId) {
        if let Some(&from_idx) = self.id_to_value.get(&from) {
            self.id_to_value.insert(into, from_idx);
            self.values[from_idx].aliases.push(into);
        } else {
            // Source not tracked yet; create a fresh value for the target.
            self.create(into, ValueKind::Mutable);
        }
    }

    /// Record that `into` captures a reference to `from`.
    fn capture(&mut self, from: IdentifierId, into: IdentifierId) {
        // Ensure both exist.
        if !self.id_to_value.contains_key(&into) {
            self.create(into, ValueKind::Mutable);
        }
        if let Some(&into_idx) = self.id_to_value.get(&into) {
            self.values[into_idx].captures.push(from);
        }
    }

    /// Mark a value as mutated.
    fn mutate(&mut self, id: IdentifierId) {
        if let Some(&idx) = self.id_to_value.get(&id) {
            self.values[idx].mutated = true;
        }
    }

    /// Mark a value as conditionally mutated.
    fn mutate_conditionally(&mut self, id: IdentifierId) {
        if let Some(&idx) = self.id_to_value.get(&id) {
            self.values[idx].conditionally_mutated = true;
        }
    }

    /// Transitively mutate: mutate the value and everything it captures.
    fn mutate_transitive(&mut self, id: IdentifierId) {
        self.mutate(id);
        // Collect captured IDs first to avoid borrow issues.
        let captured: Vec<IdentifierId> = self
            .id_to_value
            .get(&id)
            .and_then(|&idx| self.values.get(idx))
            .map(|v| v.captures.clone())
            .unwrap_or_default();
        for cap_id in captured {
            self.mutate(cap_id);
        }
    }

    /// Freeze a value.
    fn freeze(&mut self, id: IdentifierId, reason: FreezeReason) {
        if let Some(&idx) = self.id_to_value.get(&id) {
            self.values[idx].frozen = true;
            self.values[idx].freeze_reason = Some(reason);
        }
    }

    /// Check if a value is frozen.
    fn is_frozen(&self, id: IdentifierId) -> bool {
        self.id_to_value
            .get(&id)
            .and_then(|&idx| self.values.get(idx))
            .map(|v| v.frozen)
            .unwrap_or(false)
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
/// Algorithm:
/// 1. For each instruction, compute candidate effects
/// 2. Build abstract heap model (pointer graph)
/// 3. Propagate effects through the heap
/// 4. Write resolved effects back to places
pub fn infer_mutation_aliasing_effects(hir: &mut HIR) {
    // Phase 1: Compute initial effects for each instruction.
    for (_, block) in hir.blocks.iter_mut() {
        for instr in &mut block.instructions {
            let effects = compute_instruction_effects(&instr.value, &instr.lvalue);
            instr.effects = Some(effects);
        }
    }

    // Phase 2: Build abstract heap from the computed effects.
    let mut heap = AbstractHeap::new();

    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    process_effect_for_heap(&mut heap, effect);
                }
            }
        }
    }

    // Phase 3: Propagate mutations through aliases (fixpoint).
    // After building the heap, propagate transitive mutations. All captures of a mutated
    // value should also be marked as mutated. We iterate until no more changes occur.
    let mut changed = true;
    let mut iterations = 0;
    const MAX_ITERATIONS: usize = 100;

    while changed && iterations < MAX_ITERATIONS {
        changed = false;
        iterations += 1;

        // Collect propagation work to avoid borrow issues.
        let mut to_mutate: Vec<usize> = Vec::new();

        for value_idx in 0..heap.values.len() {
            if !heap.values[value_idx].mutated && !heap.values[value_idx].conditionally_mutated {
                continue;
            }

            let is_mutated = heap.values[value_idx].mutated;
            let captured: Vec<IdentifierId> = heap.values[value_idx].captures.clone();

            for cap_id in captured {
                if let Some(&cap_idx) = heap.id_to_value.get(&cap_id) {
                    if !heap.values[cap_idx].mutated && is_mutated {
                        to_mutate.push(cap_idx);
                    }
                }
            }
        }

        for idx in to_mutate {
            if !heap.values[idx].mutated {
                heap.values[idx].mutated = true;
                changed = true;
            }
        }
    }

    // Phase 4: Write effects back to all places.
    for (_, block) in hir.blocks.iter_mut() {
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

/// Process a single aliasing effect to build the heap model.
fn process_effect_for_heap(heap: &mut AbstractHeap, effect: &AliasingEffect) {
    match effect {
        AliasingEffect::Create { into, value, .. } => {
            heap.create(into.identifier.id, *value);
        }
        AliasingEffect::CreateFrom { from, into } => {
            heap.create_from(from.identifier.id, into.identifier.id);
        }
        AliasingEffect::CreateFunction { captures, function: _, into } => {
            heap.create(into.identifier.id, ValueKind::Mutable);
            for cap in captures {
                heap.capture(cap.identifier.id, into.identifier.id);
            }
        }
        AliasingEffect::Apply { receiver: _, function: _, args, into, signature } => {
            // Create a fresh value for the return.
            heap.create(into.identifier.id, ValueKind::Mutable);

            // If there's a function signature, apply parameter effects.
            if let Some(sig) = signature {
                for (arg, param_effect) in args.iter().zip(sig.params.iter()) {
                    match param_effect.effect {
                        Effect::Mutate => heap.mutate(arg.identifier.id),
                        Effect::ConditionallyMutate => heap.mutate_conditionally(arg.identifier.id),
                        Effect::Capture => {
                            heap.capture(arg.identifier.id, into.identifier.id);
                        }
                        Effect::Freeze => {
                            heap.freeze(arg.identifier.id, FreezeReason::FrozenByValue)
                        }
                        _ => {}
                    }
                    if param_effect.alias_to_return {
                        heap.alias(arg.identifier.id, into.identifier.id);
                    }
                }
            }
        }
        AliasingEffect::Assign { from, into } | AliasingEffect::Alias { from, into } => {
            heap.alias(from.identifier.id, into.identifier.id);
        }
        AliasingEffect::MaybeAlias { from, into } => {
            // Conservative: treat as alias.
            heap.alias(from.identifier.id, into.identifier.id);
        }
        AliasingEffect::Capture { from, into } => {
            heap.capture(from.identifier.id, into.identifier.id);
        }
        AliasingEffect::ImmutableCapture { from, into } => {
            // Capture but mark the captured value as frozen.
            heap.capture(from.identifier.id, into.identifier.id);
            heap.freeze(from.identifier.id, FreezeReason::FrozenByValue);
        }
        AliasingEffect::Mutate { value } => {
            heap.mutate(value.identifier.id);
        }
        AliasingEffect::MutateConditionally { value } => {
            heap.mutate_conditionally(value.identifier.id);
        }
        AliasingEffect::MutateTransitive { value } => {
            heap.mutate_transitive(value.identifier.id);
        }
        AliasingEffect::MutateTransitiveConditionally { value } => {
            heap.mutate_conditionally(value.identifier.id);
            // Also conditionally mutate captures.
            let captured: Vec<IdentifierId> = heap
                .id_to_value
                .get(&value.identifier.id)
                .and_then(|&idx| heap.values.get(idx))
                .map(|v| v.captures.clone())
                .unwrap_or_default();
            for cap_id in captured {
                heap.mutate_conditionally(cap_id);
            }
        }
        AliasingEffect::Freeze { value, reason } => {
            heap.freeze(value.identifier.id, *reason);
        }
        AliasingEffect::MutateFrozen { .. }
        | AliasingEffect::MutateGlobal { .. }
        | AliasingEffect::Impure { .. }
        | AliasingEffect::Render { .. } => {
            // These are diagnostic effects -- they don't modify the heap model.
        }
    }
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
        InstructionValue::ComputedLoad { object, property } => {
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
        InstructionValue::CallExpression { callee, args } => {
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
                        update_place(place, heap)
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
