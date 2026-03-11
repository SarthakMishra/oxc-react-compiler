# Inference Pass Gaps

> The inference passes determine mutation effects, mutable ranges, and reactivity.
> These are the core algorithmic engine driving all memoization decisions.

---

## Gap 1: infer_mutation_aliasing_effects Phases 2-3

**Upstream:** `packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingEffects.ts`
(~1,200 lines)
**Current state:** `inference/infer_mutation_aliasing_effects.rs` (33 lines) implements
Phase 1 only -- it calls `compute_instruction_effects` for each instruction. Phases 2-3
are TODO comments at lines 24-29. The aliasing effects module (`aliasing_effects.rs`, 312 lines)
that computes per-instruction effects is complete.
**What's needed:**
- **Phase 2: Abstract heap model** -- Build a pointer graph mapping each `IdentifierId` to
  an abstract value. Track aliases (same abstract value), captures (indirect reference),
  and creates (new abstract value). This is the "abstract interpretation" core.
  - Data structure: `FxHashMap<IdentifierId, AbstractValue>` where `AbstractValue` tracks
    `ValueKind`, alias set, and frozen state
  - Process `AliasingEffect::Alias` to merge abstract values
  - Process `AliasingEffect::Capture` to create indirect edges
  - Process `AliasingEffect::Create/CreateFrom/CreateFunction` to allocate new abstract values
  - Process `AliasingEffect::Apply` to model function calls (look up signatures in shape system)
- **Phase 3: Fixpoint propagation** -- Iterate over all instructions until effects stabilize:
  - For each `Mutate/MutateConditionally` effect, propagate mutation through aliases
  - For each `MutateTransitive` effect, propagate through captures too
  - For each `Freeze` effect, mark the abstract value as frozen
  - Detect `MutateFrozen` violations (mutating a frozen value)
  - Record final `Effect` on each `Place` (the `effect` field on Place)
- **Phase 4: Write effects back** -- Set `place.effect` for every operand Place in every
  instruction based on the resolved abstract heap state
**Depends on:** None (the Phase 1 infrastructure and AliasingEffect types are complete)

---

## Gap 2: infer_mutation_aliasing_ranges Transitive Tracking

**Upstream:** `packages/babel-plugin-react-compiler/src/Inference/InferMutationAliasingRanges.ts`
(~200 lines)
**Current state:** `inference/infer_mutation_aliasing_ranges.rs` (33 lines) sets a basic
`MutableRange { start: id, end: id+1 }` for each instruction. The TODO at line 28 describes
what's missing.
**What's needed:**
- Build a map of `IdentifierId -> Vec<InstructionId>` for all mutation sites (from the
  effects computed in Gap 1 above)
- For each identifier, compute range as `[creation_instruction, last_mutation_instruction]`
- Handle transitive mutations: if A aliases B and B is mutated at instruction N, then A's
  mutable range must extend to N
- Handle function captures: if a closure captures A and mutates it, A's mutable range must
  extend to the closure's last use
- This pass is critical because `infer_reactive_scope_variables` uses mutable ranges to
  decide which identifiers belong in the same reactive scope
**Depends on:** Gap 1 (needs complete effects to know all mutation sites)
