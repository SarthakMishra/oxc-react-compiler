# SSA Conversion

> Convert HIR to SSA form for downstream optimization and analysis passes.
> Upstream: `src/SSA/EnterSSA.ts`, `src/SSA/EliminateRedundantPhi.ts`
> Rust modules: `crates/oxc_react_compiler/src/ssa/enter_ssa.rs`, `eliminate_redundant_phi.rs`

---

### Gap 1: EnterSSA

**Upstream:** `src/SSA/EnterSSA.ts`
**Pipeline position:** Pass #8, Phase 2
**Current state:** `ssa/enter_ssa.rs` is a stub.
**What's needed:**

Standard SSA construction algorithm:

1. **Compute dominance frontiers** for the CFG
   - Build dominator tree from entry block
   - Compute dominance frontier for each block
2. **Insert phi nodes** at dominance frontiers for each variable
   - For each variable defined in block B, insert phi at every block in DF(B)
   - Iterative phi insertion until fixpoint
3. **Rename identifiers** using the SSA numbering
   - Walk the dominator tree
   - For each definition, create a new SSA name (fresh `IdentifierId`)
   - Update all uses to refer to the correct SSA name
   - Fill in phi node operands based on predecessor blocks
4. **Update `Phi` structs** on each `BasicBlock`
   - `Phi { id, place, operands: FxHashMap<BlockId, Place> }`

Algorithm: Standard Cytron et al. SSA construction.

Input: HIR with non-SSA identifiers (same `IdentifierId` for all uses of a variable)
Output: HIR with SSA identifiers (unique `IdentifierId` per definition point, phi nodes at join points)

Key implementation detail: The upstream implementation operates on the HIR block structure directly, modifying identifier IDs in-place and adding phi nodes to blocks.

**Depends on:** HIR types (complete), BuildHIR (to have valid HIR to transform)

---

### Gap 2: EliminateRedundantPhi

**Upstream:** `src/SSA/EliminateRedundantPhi.ts`
**Pipeline position:** Pass #9, Phase 2
**Current state:** `ssa/eliminate_redundant_phi.rs` is a stub.
**What's needed:**

Remove phi nodes that are trivially redundant:

- A phi is redundant if all operands refer to the same value (or to the phi itself)
  - `phi(x, x, x)` -> replace all uses of phi result with `x`
  - `phi(x, phi_self)` -> replace with `x` (self-referencing)
- Iterative: removing one redundant phi may make others redundant
- Walk all blocks, check each phi, replace uses if redundant
- Remove the phi from the block's phi set
- Update all instructions that referenced the phi's output to use the simplified value

This is a standard SSA optimization pass. The upstream implementation is straightforward.

**Depends on:** Gap 1 (EnterSSA)
