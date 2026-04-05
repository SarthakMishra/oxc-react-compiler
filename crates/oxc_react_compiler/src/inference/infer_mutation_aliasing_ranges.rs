// Graph-based BFS mutation propagation for computing mutable ranges.
//
// Upstream: InferMutationAliasingRanges.ts
//
// The algorithm:
// 1. Build a directed alias/capture graph from AliasingEffect variants
// 2. Collect all mutations as deferred work items
// 3. Process each mutation via BFS, propagating range extensions through
//    alias, capture, createdFrom, and maybeAlias edges
// 4. Write the computed ranges back to HIR identifiers

use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

use crate::hir::types::{
    AliasingEffect, BlockId, Effect, HIR, IdentifierId, InstructionId, InstructionValue,
    MutableRange, Place, Terminal,
};

// ---------------------------------------------------------------------------
// MutationKind — ordered so we can skip re-visits with weaker kinds
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MutationKind {
    Conditional = 1,
    Definite = 2,
}

// ---------------------------------------------------------------------------
// Graph edge types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum EdgeKind {
    Alias,
    Capture,
    MaybeAlias,
}

/// A forward edge from one node to another, tagged with the temporal index
/// at which the edge was created and the kind of relationship.
#[derive(Clone, Copy)]
struct Edge {
    index: u32,
    target: IdentifierId,
    kind: EdgeKind,
}

/// A backward edge reference: source node and temporal index.
type BackEdge = (IdentifierId, u32);

// ---------------------------------------------------------------------------
// Graph node
// ---------------------------------------------------------------------------

struct Node {
    /// Forward edges (alias/capture/maybeAlias leaving this node).
    /// Appended in monotonically increasing index order.
    edges: Vec<Edge>,
    /// Backward alias edges (Alias/Assign effects where this node is `into`).
    aliases: Vec<BackEdge>,
    /// Backward createdFrom edges.
    created_from: Vec<BackEdge>,
    /// Backward capture edges.
    captures: Vec<BackEdge>,
    /// Backward maybeAlias edges.
    maybe_aliases: Vec<BackEdge>,
}

impl Node {
    fn new() -> Self {
        Self {
            edges: Vec::new(),
            aliases: Vec::new(),
            created_from: Vec::new(),
            captures: Vec::new(),
            maybe_aliases: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Backward edge classification
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum BackwardEdgeKind {
    Alias,
    CreatedFrom,
    Capture,
    MaybeAlias,
}

// ---------------------------------------------------------------------------
// Aliasing graph
// ---------------------------------------------------------------------------

struct AliasingGraph {
    nodes: FxHashMap<IdentifierId, Node>,
}

impl AliasingGraph {
    fn new() -> Self {
        Self { nodes: FxHashMap::default() }
    }

    fn ensure_node(&mut self, id: IdentifierId) -> &mut Node {
        self.nodes.entry(id).or_insert_with(Node::new)
    }

    /// Register a fresh creation (no edges).
    fn create(&mut self, id: IdentifierId) {
        self.ensure_node(id);
    }

    /// Add a forward edge and backward reference between two nodes.
    fn add_edge(
        &mut self,
        index: u32,
        from: IdentifierId,
        into: IdentifierId,
        kind: EdgeKind,
        backward: BackwardEdgeKind,
    ) {
        self.ensure_node(from).edges.push(Edge { index, target: into, kind });
        let into_node = self.ensure_node(into);
        match backward {
            BackwardEdgeKind::Alias => into_node.aliases.push((from, index)),
            BackwardEdgeKind::CreatedFrom => into_node.created_from.push((from, index)),
            BackwardEdgeKind::Capture => into_node.captures.push((from, index)),
            BackwardEdgeKind::MaybeAlias => into_node.maybe_aliases.push((from, index)),
        }
    }

    /// CreateFrom: forward edge from `from` → `into`, backward createdFrom on `into`.
    fn create_from(&mut self, index: u32, from: IdentifierId, into: IdentifierId) {
        self.add_edge(index, from, into, EdgeKind::Alias, BackwardEdgeKind::CreatedFrom);
    }

    /// Assign / Alias: forward edge from `from` → `into`, backward alias on `into`.
    fn assign(&mut self, index: u32, from: IdentifierId, into: IdentifierId) {
        self.add_edge(index, from, into, EdgeKind::Alias, BackwardEdgeKind::Alias);
    }

    /// Capture: forward edge from `from` → `into`, backward capture on `into`.
    fn capture(&mut self, index: u32, from: IdentifierId, into: IdentifierId) {
        self.add_edge(index, from, into, EdgeKind::Capture, BackwardEdgeKind::Capture);
    }

    /// MaybeAlias: forward edge from `from` → `into`, backward maybeAlias on `into`.
    fn maybe_alias(&mut self, index: u32, from: IdentifierId, into: IdentifierId) {
        self.add_edge(index, from, into, EdgeKind::MaybeAlias, BackwardEdgeKind::MaybeAlias);
    }
}

// ---------------------------------------------------------------------------
// Deferred mutation
// ---------------------------------------------------------------------------

struct PendingMutation {
    index: u32,
    instr_id: InstructionId,
    place_id: IdentifierId,
    transitive: bool,
    kind: MutationKind,
}

// ---------------------------------------------------------------------------
// BFS mutation propagation
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Direction {
    Backwards,
    Forwards,
}

struct WorkItem {
    place: IdentifierId,
    transitive: bool,
    direction: Direction,
    kind: MutationKind,
}

/// Reusable BFS buffers to avoid per-mutation allocation.
struct BfsBuffers {
    seen: FxHashMap<IdentifierId, MutationKind>,
    queue: VecDeque<WorkItem>,
}

impl BfsBuffers {
    fn new() -> Self {
        Self { seen: FxHashMap::default(), queue: VecDeque::new() }
    }

    fn clear(&mut self) {
        self.seen.clear();
        self.queue.clear();
    }
}

/// Propagate a mutation from `start` through the alias/capture graph via BFS.
///
/// For each reachable node, extends `mutableRange.end` to `end_instr`.
/// Upstream: `mutate()` in `InferMutationAliasingRanges.ts`.
///
/// PERF: Takes reusable BFS buffers to avoid per-mutation HashMap/VecDeque allocation.
fn mutate(
    graph: &AliasingGraph,
    ranges: &mut FxHashMap<IdentifierId, MutableRange>,
    start: IdentifierId,
    mutation_index: u32,
    end_instr: InstructionId,
    transitive: bool,
    start_kind: MutationKind,
    phi_ids: &FxHashSet<IdentifierId>,
    bufs: &mut BfsBuffers,
) {
    bufs.clear();
    bufs.queue.push_back(WorkItem {
        place: start,
        transitive,
        direction: Direction::Backwards,
        kind: start_kind,
    });

    while let Some(item) = bufs.queue.pop_front() {
        // Dedup: skip if already visited with equal or stronger kind
        if let Some(&prev) = bufs.seen.get(&item.place)
            && prev >= item.kind
        {
            continue;
        }
        bufs.seen.insert(item.place, item.kind);

        // Extend mutable range end (start is set by creation_map in Phase 3)
        if let Some(range) = ranges.get_mut(&item.place)
            && end_instr > range.end
        {
            range.end = end_instr;
        }

        let Some(node) = graph.nodes.get(&item.place) else { continue };

        // Forward edges: only follow edges created before the mutation
        for edge in &node.edges {
            if edge.index >= mutation_index {
                continue; // skip edges created after the mutation
            }
            let child_kind = if matches!(edge.kind, EdgeKind::MaybeAlias) {
                MutationKind::Conditional
            } else {
                item.kind
            };
            bufs.queue.push_back(WorkItem {
                place: edge.target,
                transitive: item.transitive,
                direction: Direction::Forwards,
                kind: child_kind,
            });
        }

        // Backward createdFrom: always follow, transitions to transitive=true
        for &(from, when) in &node.created_from {
            if when >= mutation_index {
                continue;
            }
            bufs.queue.push_back(WorkItem {
                place: from,
                transitive: true,
                direction: Direction::Backwards,
                kind: item.kind,
            });
        }

        // Backward aliases and maybeAliases: skip if arriving at a phi via forward edge
        // (upstream: `direction === 'backwards'` check for phi nodes)
        let skip_backward_aliases =
            item.direction == Direction::Forwards && phi_ids.contains(&item.place);

        if !skip_backward_aliases {
            for &(alias, when) in &node.aliases {
                if when >= mutation_index {
                    continue;
                }
                bufs.queue.push_back(WorkItem {
                    place: alias,
                    transitive: item.transitive,
                    direction: Direction::Backwards,
                    kind: item.kind,
                });
            }
            for &(alias, when) in &node.maybe_aliases {
                if when >= mutation_index {
                    continue;
                }
                bufs.queue.push_back(WorkItem {
                    place: alias,
                    transitive: item.transitive,
                    direction: Direction::Backwards,
                    kind: MutationKind::Conditional,
                });
            }
        }

        // Backward captures: only if transitive
        if item.transitive {
            for &(cap, when) in &node.captures {
                if when >= mutation_index {
                    continue;
                }
                bufs.queue.push_back(WorkItem {
                    place: cap,
                    transitive: item.transitive,
                    direction: Direction::Backwards,
                    kind: item.kind,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Compute mutable ranges for all identifiers using graph-based BFS propagation.
///
/// Upstream: `InferMutationAliasingRanges.ts`
///
/// 1. Builds a directed alias/capture graph from `AliasingEffect` annotations
/// 2. Defers all mutation effects
/// 3. Processes mutations via BFS, extending ranges through the graph
/// 4. Writes ranges back to HIR identifiers
pub fn infer_mutation_aliasing_ranges(hir: &mut HIR, returns_id: Option<IdentifierId>) {
    let mut graph = AliasingGraph::new();
    let mut phi_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    // PERF: Pre-size maps based on instruction count to reduce rehashing.
    let instr_count_hint: usize = hir.blocks.iter().map(|(_, b)| b.instructions.len()).sum();
    let mut ranges: FxHashMap<IdentifierId, MutableRange> =
        FxHashMap::with_capacity_and_hasher(instr_count_hint, rustc_hash::FxBuildHasher);
    let mut mutations: Vec<PendingMutation> = Vec::new();
    let mut creation_map: FxHashMap<IdentifierId, InstructionId> =
        FxHashMap::with_capacity_and_hasher(instr_count_hint, rustc_hash::FxBuildHasher);

    // Pending phi operands for back-edges (predecessor block not yet visited)
    let mut pending_phis: FxHashMap<BlockId, Vec<(u32, IdentifierId, IdentifierId)>> =
        FxHashMap::default();

    let mut index: u32 = 0;
    let mut seen_blocks: FxHashSet<BlockId> = FxHashSet::default();

    // Fix 4: Create fn.returns node in graph (upstream creates it alongside params/context)
    if let Some(ret_id) = returns_id {
        graph.create(ret_id);
        ranges.insert(ret_id, MutableRange { start: InstructionId(0), end: InstructionId(0) });
    }

    // Phase 1: Build the graph and collect mutations
    for (block_id, block) in &hir.blocks {
        // Process phi nodes
        for phi in &block.phis {
            let phi_id = phi.place.identifier.id;
            phi_ids.insert(phi_id);
            graph.create(phi_id);
            ranges.insert(phi_id, phi.place.identifier.mutable_range);

            for (pred_block_id, operand) in &phi.operands {
                let from_id = operand.identifier.id;
                ranges.entry(from_id).or_insert(operand.identifier.mutable_range);

                if seen_blocks.contains(pred_block_id) {
                    // Predecessor already visited: wire up immediately
                    graph.assign(index, from_id, phi_id);
                } else {
                    // Back-edge: defer until predecessor block is visited
                    pending_phis.entry(*pred_block_id).or_default().push((index, from_id, phi_id));
                }
                index += 1;
            }
        }

        seen_blocks.insert(*block_id);

        // Process instructions
        for instr in &block.instructions {
            let lv_id = instr.lvalue.identifier.id;
            ranges.entry(lv_id).or_insert(instr.lvalue.identifier.mutable_range);
            creation_map.entry(lv_id).or_insert(instr.id);

            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Create { into, .. } => {
                            graph.create(into.identifier.id);
                            creation_map.entry(into.identifier.id).or_insert(instr.id);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            // Create does NOT consume an index slot
                        }
                        AliasingEffect::CreateFunction { into, captures, .. } => {
                            graph.create(into.identifier.id);
                            creation_map.entry(into.identifier.id).or_insert(instr.id);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            // Wire up capture edges from each captured place → function
                            for cap in captures {
                                ranges
                                    .entry(cap.identifier.id)
                                    .or_insert(cap.identifier.mutable_range);
                                graph.capture(index, cap.identifier.id, into.identifier.id);
                                index += 1;
                            }
                        }
                        AliasingEffect::CreateFrom { from, into } => {
                            ranges
                                .entry(from.identifier.id)
                                .or_insert(from.identifier.mutable_range);
                            creation_map.entry(into.identifier.id).or_insert(instr.id);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            graph.create_from(index, from.identifier.id, into.identifier.id);
                            index += 1;
                        }
                        AliasingEffect::Assign { from, into } => {
                            ranges
                                .entry(from.identifier.id)
                                .or_insert(from.identifier.mutable_range);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            graph.assign(index, from.identifier.id, into.identifier.id);
                            index += 1;
                        }
                        AliasingEffect::Alias { from, into } => {
                            ranges
                                .entry(from.identifier.id)
                                .or_insert(from.identifier.mutable_range);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            graph.assign(index, from.identifier.id, into.identifier.id);
                            index += 1;
                        }
                        AliasingEffect::MaybeAlias { from, into } => {
                            ranges
                                .entry(from.identifier.id)
                                .or_insert(from.identifier.mutable_range);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            graph.maybe_alias(index, from.identifier.id, into.identifier.id);
                            index += 1;
                        }
                        AliasingEffect::Capture { from, into } => {
                            ranges
                                .entry(from.identifier.id)
                                .or_insert(from.identifier.mutable_range);
                            ranges
                                .entry(into.identifier.id)
                                .or_insert(into.identifier.mutable_range);
                            graph.capture(index, from.identifier.id, into.identifier.id);
                            index += 1;
                        }
                        AliasingEffect::Mutate { value, .. } => {
                            ranges
                                .entry(value.identifier.id)
                                .or_insert(value.identifier.mutable_range);
                            mutations.push(PendingMutation {
                                index,
                                instr_id: instr.id,
                                place_id: value.identifier.id,
                                transitive: false,
                                kind: MutationKind::Definite,
                            });
                            index += 1;
                        }
                        AliasingEffect::MutateConditionally { value } => {
                            ranges
                                .entry(value.identifier.id)
                                .or_insert(value.identifier.mutable_range);
                            mutations.push(PendingMutation {
                                index,
                                instr_id: instr.id,
                                place_id: value.identifier.id,
                                transitive: false,
                                kind: MutationKind::Conditional,
                            });
                            index += 1;
                        }
                        AliasingEffect::MutateTransitive { value } => {
                            ranges
                                .entry(value.identifier.id)
                                .or_insert(value.identifier.mutable_range);
                            mutations.push(PendingMutation {
                                index,
                                instr_id: instr.id,
                                place_id: value.identifier.id,
                                transitive: true,
                                kind: MutationKind::Definite,
                            });
                            index += 1;
                        }
                        AliasingEffect::MutateTransitiveConditionally { value } => {
                            ranges
                                .entry(value.identifier.id)
                                .or_insert(value.identifier.mutable_range);
                            mutations.push(PendingMutation {
                                index,
                                instr_id: instr.id,
                                place_id: value.identifier.id,
                                transitive: true,
                                kind: MutationKind::Conditional,
                            });
                            index += 1;
                        }
                        // Effects that don't establish graph edges or mutations
                        AliasingEffect::ImmutableCapture { .. }
                        | AliasingEffect::Freeze { .. }
                        | AliasingEffect::MutateFrozen { .. }
                        | AliasingEffect::MutateGlobal { .. }
                        | AliasingEffect::Impure { .. }
                        | AliasingEffect::Render { .. } => {}
                        // Apply effects are fully resolved by
                        // infer_mutation_aliasing_effects (Pass 16) into concrete
                        // Mutate/Capture/CreateFrom/etc. effects before this pass
                        // runs. Any remaining Apply here is a no-op (matches upstream's
                        // invariant that Apply must be resolved before range inference).
                        AliasingEffect::Apply { .. } => {}
                    }
                }
            }
        }

        // Process deferred phi operands for this block (back-edges from later blocks)
        if let Some(pending) = pending_phis.remove(block_id) {
            for (phi_index, from_id, into_id) in pending {
                graph.assign(phi_index, from_id, into_id);
            }
        }

        // Fix 4: Assign return value to fn.returns
        // Upstream: if terminal is return, assign return value → fn.returns
        if let Some(ret_id) = returns_id
            && let Terminal::Return { value, .. } = &block.terminal
        {
            let from_id = value.identifier.id;
            ranges.entry(from_id).or_insert(value.identifier.mutable_range);
            graph.assign(index, from_id, ret_id);
            index += 1;
        }

        // Fix 5: Process MaybeThrow and Return terminal effects
        // Upstream: if terminal is maybe-throw or return with effects,
        // process Alias effects as graph assignments.
        let terminal_effects: Option<&Vec<AliasingEffect>> = match &block.terminal {
            Terminal::MaybeThrow { effects, .. } => effects.as_ref(),
            Terminal::Return { effects, .. } => effects.as_ref(),
            _ => None,
        };
        if let Some(effects) = terminal_effects {
            for effect in effects {
                if let AliasingEffect::Alias { from, into } = effect {
                    ranges.entry(from.identifier.id).or_insert(from.identifier.mutable_range);
                    ranges.entry(into.identifier.id).or_insert(into.identifier.mutable_range);
                    graph.assign(index, from.identifier.id, into.identifier.id);
                    index += 1;
                }
                // Upstream: Freeze effects are just validated (invariant check),
                // not processed as graph edges. We skip them.
            }
        }
    }

    // Phase 2: Process all mutations against the completed graph
    // PERF: Reuse BFS buffers across mutations to avoid per-mutation allocation.
    let mut bfs_bufs = BfsBuffers::new();
    for m in &mutations {
        let end = InstructionId(m.instr_id.0 + 1);
        mutate(
            &graph,
            &mut ranges,
            m.place_id,
            m.index,
            end,
            m.transitive,
            m.kind,
            &phi_ids,
            &mut bfs_bufs,
        );
    }

    // Phase 3: Write mutable ranges back to HIR lvalue identifiers
    // Use creation_map for start, and extend end to cover last_use + BFS mutations
    for (_, block) in &mut hir.blocks {
        for phi in &mut block.phis {
            if let Some(&range) = ranges.get(&phi.place.identifier.id) {
                phi.place.identifier.mutable_range = range;
            }
        }
        for instr in &mut block.instructions {
            let id = instr.lvalue.identifier.id;
            let start = creation_map.get(&id).copied().unwrap_or(instr.id);
            let mut end = InstructionId(start.0 + 1);

            // Extend to BFS-computed mutation end
            if let Some(&range) = ranges.get(&id)
                && range.end > end
            {
                end = range.end;
            }

            instr.lvalue.identifier.mutable_range = MutableRange { start, end };
        }
    }

    // Phase 4: Annotate Place.effect on all operands and lvalues.
    // Upstream: "Part 2" of inferMutationAliasingRanges.
    //
    // Build a set of all identifiers that are mutated (have range.end > range.start + 1
    // or appear in the mutations list).
    annotate_place_effects(hir);
}

// ---------------------------------------------------------------------------
// Place.effect annotation
// ---------------------------------------------------------------------------

/// Annotate `Place.effect` on all operands, lvalues, and terminals.
///
/// Upstream: "Part 2" of `inferMutationAliasingRanges()`.
///
/// Assignment rules:
/// - Phi places: `Store`
/// - Phi operands: `Capture` if the phi is mutated after creation, else `Read`
/// - Instruction lvalues: `ConditionallyMutate` (default)
/// - Instruction operands: `Read` (default), refined by instruction effects
/// - Return terminal: `Freeze` for non-FE, `Read` for FE (handled upstream in effects pass)
///
/// Effect-based refinements override the defaults:
/// - Assign/Alias/Capture/CreateFrom/MaybeAlias target → `Store`, source → `Capture` if target
///   is later mutated, else `Read`
/// - Mutate → `Store`
/// - MutateTransitive/Conditional variants → `ConditionallyMutate`
/// - Freeze → `Freeze`
fn annotate_place_effects(hir: &mut HIR) {
    for (_, block) in &mut hir.blocks {
        // Upstream: firstInstructionIdOfBlock = block.instructions[0]?.id ?? block.terminal.id
        let first_instr_id_of_block = first_instruction_id_of_block(block);

        // Annotate phi nodes
        for phi in &mut block.phis {
            phi.place.effect = Effect::Store;
            // Upstream: isPhiMutatedAfterCreation = mutableRange.end > firstInstructionId
            let phi_mutated = phi.place.identifier.mutable_range.end > first_instr_id_of_block;
            for (_, operand) in &mut phi.operands {
                operand.effect = if phi_mutated { Effect::Capture } else { Effect::Read };
            }
            // Fix 2: Phi mutableRange.start fixup
            // Upstream: if phi is mutated after creation and start==0, set start to
            // firstInstructionIdOfBlock - 1
            if phi_mutated && phi.place.identifier.mutable_range.start == InstructionId(0) {
                phi.place.identifier.mutable_range.start =
                    InstructionId(first_instr_id_of_block.0.saturating_sub(1));
            }
        }

        // Annotate instructions
        for instr in &mut block.instructions {
            // Default lvalue effect
            instr.lvalue.effect = Effect::ConditionallyMutate;
            // Fix 3b: Lvalue mutableRange.start fixup
            // Upstream: if lvalue.mutableRange.start == 0, set to instr.id
            if instr.lvalue.identifier.mutable_range.start == InstructionId(0) {
                instr.lvalue.identifier.mutable_range.start = instr.id;
            }
            // Upstream: if lvalue.mutableRange.end == 0, set to max(instr.id + 1, end)
            if instr.lvalue.identifier.mutable_range.end == InstructionId(0) {
                instr.lvalue.identifier.mutable_range.end = InstructionId(instr.id.0 + 1);
            }

            // Build per-operand effect map from instruction effects
            let mut operand_effects: FxHashMap<IdentifierId, Effect> = FxHashMap::default();

            if let Some(ref effects) = instr.effects {
                for effect in effects {
                    match effect {
                        AliasingEffect::Assign { from, into }
                        | AliasingEffect::Alias { from, into }
                        | AliasingEffect::CreateFrom { from, into }
                        | AliasingEffect::MaybeAlias { from, into } => {
                            // Upstream: isMutatedOrReassigned = mutableRange.end > instr.id
                            let is_mutated_or_reassigned =
                                into.identifier.mutable_range.end > instr.id;
                            // Target gets Store
                            operand_effects.insert(into.identifier.id, Effect::Store);
                            // Source: Capture if target is later mutated, else Read
                            let source_effect = if is_mutated_or_reassigned {
                                Effect::Capture
                            } else {
                                Effect::Read
                            };
                            // Only upgrade, don't downgrade
                            operand_effects
                                .entry(from.identifier.id)
                                .and_modify(|e| {
                                    if effect_priority(source_effect) > effect_priority(*e) {
                                        *e = source_effect;
                                    }
                                })
                                .or_insert(source_effect);
                        }
                        AliasingEffect::Capture { from, into } => {
                            let is_mutated_or_reassigned =
                                into.identifier.mutable_range.end > instr.id;
                            operand_effects.insert(into.identifier.id, Effect::Store);
                            let source_effect = if is_mutated_or_reassigned {
                                Effect::Capture
                            } else {
                                Effect::Read
                            };
                            operand_effects
                                .entry(from.identifier.id)
                                .and_modify(|e| {
                                    if effect_priority(source_effect) > effect_priority(*e) {
                                        *e = source_effect;
                                    }
                                })
                                .or_insert(source_effect);
                        }
                        AliasingEffect::Mutate { value, .. } => {
                            operand_effects.insert(value.identifier.id, Effect::Store);
                        }
                        AliasingEffect::MutateTransitive { value }
                        | AliasingEffect::MutateConditionally { value }
                        | AliasingEffect::MutateTransitiveConditionally { value } => {
                            operand_effects
                                .insert(value.identifier.id, Effect::ConditionallyMutate);
                        }
                        AliasingEffect::Freeze { value, .. } => {
                            operand_effects.insert(value.identifier.id, Effect::Freeze);
                        }
                        AliasingEffect::ImmutableCapture { from, .. } => {
                            operand_effects.entry(from.identifier.id).or_insert(Effect::Read);
                        }
                        _ => {}
                    }
                }
            }

            // Upstream: apply lvalue effects from operandEffects map first
            if let Some(&eff) = operand_effects.get(&instr.lvalue.identifier.id) {
                instr.lvalue.effect = eff;
            }

            // Fix 3: Operand mutableRange.start fixup
            // Upstream: for each operand, if range.end > instr.id and start==0,
            // set start = instr.id. This ensures operands used before their mutation
            // get a proper range start.
            apply_operand_start_fixup(&mut instr.value, instr.id);

            // Apply effects to operand places
            apply_operand_effects(&mut instr.value, &operand_effects);

            // Fix 1: StoreContext range extension
            // Upstream: if instruction is StoreContext and the stored value's
            // mutableRange.end <= instr.id, extend it to instr.id + 1.
            // This ensures context variables have their ranges extended when stored.
            if let InstructionValue::StoreContext { value, .. } = &mut instr.value
                && value.identifier.mutable_range.end <= instr.id
            {
                value.identifier.mutable_range.end = InstructionId(instr.id.0 + 1);
            }
        }

        // Annotate terminal
        match &mut block.terminal {
            Terminal::Return { value, .. } => {
                // For function expressions, return value is Read (not frozen).
                // For top-level functions, the freeze is already handled by
                // InferMutationAliasingEffects. We default to Read here.
                if value.effect == Effect::Unknown {
                    value.effect = Effect::Read;
                }
            }
            Terminal::Throw { value } => {
                value.effect = Effect::Read;
            }
            Terminal::If { test, .. } | Terminal::Branch { test, .. } => {
                test.effect = Effect::Read;
            }
            _ => {}
        }
    }
}

/// Apply per-operand effects to the operand places in an instruction value.
fn apply_operand_effects(
    value: &mut InstructionValue,
    operand_effects: &FxHashMap<IdentifierId, Effect>,
) {
    let update = |place: &mut Place| {
        if let Some(&effect) = operand_effects.get(&place.identifier.id) {
            place.effect = effect;
        } else {
            // Default: Read
            if place.effect == Effect::Unknown {
                place.effect = Effect::Read;
            }
        }
    };

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
            update(place);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            update(iterator);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            update(lvalue);
            update(value);
        }
        InstructionValue::StoreGlobal { value: place, .. } => {
            update(place);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            update(left);
            update(right);
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            update(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            update(object);
            update(value);
        }
        InstructionValue::ComputedLoad { object, property, .. }
        | InstructionValue::ComputedDelete { object, property } => {
            update(object);
            update(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            update(object);
            update(property);
            update(value);
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            update(callee);
            for arg in args.iter_mut() {
                update(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            update(receiver);
            for arg in args.iter_mut() {
                update(arg);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties.iter_mut() {
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &mut prop.key {
                    update(p);
                }
                update(&mut prop.value);
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements.iter_mut() {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => update(p),
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            update(tag);
            for prop in props.iter_mut() {
                update(&mut prop.value);
            }
            for child in children.iter_mut() {
                update(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children.iter_mut() {
                update(child);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for expr in subexpressions.iter_mut() {
                update(expr);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, .. } => {
            update(tag);
        }
        InstructionValue::Destructure { value, .. } => {
            update(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            update(decl);
            for dep in deps.iter_mut() {
                update(dep);
            }
        }
        InstructionValue::FunctionExpression { lowered_func, .. } => {
            for ctx_place in &mut lowered_func.context {
                update(ctx_place);
            }
        }
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::DeclareLocal { .. }
        | InstructionValue::DeclareContext { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

/// Compute the first instruction ID of a block for phi start fixup.
/// Upstream: `block.instructions.at(0)?.id ?? block.terminal.id`
/// We don't have terminal IDs, so if block has no instructions, use
/// last_instruction_id + 1 as an approximation (terminal would follow
/// the last instruction).
fn first_instruction_id_of_block(block: &crate::hir::types::BasicBlock) -> InstructionId {
    if let Some(first) = block.instructions.first() {
        first.id
    } else {
        // Empty block -- use InstructionId(0) as fallback.
        // In practice, blocks always have at least one instruction.
        InstructionId(0)
    }
}

/// Fix 3: Apply operand mutableRange.start fixup.
/// Upstream: for each operand, if mutableRange.end > instr.id and start==0,
/// set start = instr.id.
fn apply_operand_start_fixup(value: &mut InstructionValue, instr_id: InstructionId) {
    let fixup = |place: &mut Place| {
        if place.identifier.mutable_range.end > instr_id
            && place.identifier.mutable_range.start == InstructionId(0)
        {
            place.identifier.mutable_range.start = instr_id;
        }
    };

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
            fixup(place);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            fixup(iterator);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            fixup(lvalue);
            fixup(value);
        }
        InstructionValue::StoreGlobal { value: place, .. } => {
            fixup(place);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            fixup(left);
            fixup(right);
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            fixup(object);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            fixup(object);
            fixup(value);
        }
        InstructionValue::ComputedLoad { object, property, .. }
        | InstructionValue::ComputedDelete { object, property } => {
            fixup(object);
            fixup(property);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            fixup(object);
            fixup(property);
            fixup(value);
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            fixup(callee);
            for arg in args.iter_mut() {
                fixup(arg);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            fixup(receiver);
            for arg in args.iter_mut() {
                fixup(arg);
            }
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties.iter_mut() {
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &mut prop.key {
                    fixup(p);
                }
                fixup(&mut prop.value);
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements.iter_mut() {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => fixup(p),
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            fixup(tag);
            for prop in props.iter_mut() {
                fixup(&mut prop.value);
            }
            for child in children.iter_mut() {
                fixup(child);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children.iter_mut() {
                fixup(child);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for expr in subexpressions.iter_mut() {
                fixup(expr);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, .. } => {
            fixup(tag);
        }
        InstructionValue::Destructure { value, .. } => {
            fixup(value);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            fixup(decl);
            for dep in deps.iter_mut() {
                fixup(dep);
            }
        }
        InstructionValue::FunctionExpression { lowered_func, .. } => {
            for ctx_place in &mut lowered_func.context {
                fixup(ctx_place);
            }
        }
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::ObjectMethod { .. }
        | InstructionValue::DeclareLocal { .. }
        | InstructionValue::DeclareContext { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. } => {}
    }
}

/// Priority ordering for effects (higher = more specific/important).
fn effect_priority(e: Effect) -> u8 {
    match e {
        Effect::Unknown => 0,
        Effect::Read => 1,
        Effect::Capture => 2,
        Effect::ConditionallyMutate => 3,
        Effect::ConditionallyMutateIterator => 3,
        Effect::Freeze => 4,
        Effect::Store => 5,
        Effect::Mutate => 6,
    }
}

/// Stamp `identifier.last_use` on every lvalue identifier in the HIR.
///
/// For each identifier that appears as an operand in some instruction, this
/// records the maximum instruction ID at which it is used. This is separate
/// from `mutable_range` (which tracks only mutation propagation) and is used
/// by scope inference to decide whether a call result escapes its definition
/// site (i.e., is used somewhere after the instruction that creates it).
///
/// Must run after `infer_mutation_aliasing_ranges` (which computes narrow
/// mutation-only `mutable_range`) and before `infer_reactive_scope_variables`
/// (which reads `last_use`).
pub fn annotate_last_use(hir: &mut HIR) {
    let mut last_use_map: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();

    // Collect last-use sites from instructions and terminals
    // PERF: Use a callback pattern instead of collecting IDs into a Vec per instruction.
    for (_, block) in &hir.blocks {
        for instr in &block.instructions {
            let instr_id = instr.id;
            visit_operand_ids(&instr.value, &mut |op_id| {
                let entry = last_use_map.entry(op_id).or_insert(instr_id);
                if instr_id > *entry {
                    *entry = instr_id;
                }
            });
        }

        // Track terminal uses
        let terminal_id = InstructionId(block.instructions.last().map_or(0, |i| i.id.0) + 1);
        match &block.terminal {
            crate::hir::types::Terminal::Return { value, .. }
            | crate::hir::types::Terminal::Throw { value } => {
                let entry = last_use_map.entry(value.identifier.id).or_insert(terminal_id);
                if terminal_id > *entry {
                    *entry = terminal_id;
                }
            }
            crate::hir::types::Terminal::If { test, .. }
            | crate::hir::types::Terminal::Branch { test, .. } => {
                let entry = last_use_map.entry(test.identifier.id).or_insert(terminal_id);
                if terminal_id > *entry {
                    *entry = terminal_id;
                }
            }
            _ => {}
        }
    }

    // Write last_use back to lvalue identifiers
    for (_, block) in &mut hir.blocks {
        for instr in &mut block.instructions {
            let id = instr.lvalue.identifier.id;
            if let Some(&last_use) = last_use_map.get(&id) {
                instr.lvalue.identifier.last_use = last_use;
            }
        }
        for phi in &mut block.phis {
            let id = phi.place.identifier.id;
            if let Some(&last_use) = last_use_map.get(&id) {
                phi.place.identifier.last_use = last_use;
            }
        }
    }
}

/// PERF: Visit operand IDs via callback to avoid per-instruction Vec allocation.
fn visit_operand_ids(value: &InstructionValue, visitor: &mut dyn FnMut(IdentifierId)) {
    match value {
        InstructionValue::LoadLocal { place } | InstructionValue::LoadContext { place } => {
            visitor(place.identifier.id);
        }
        InstructionValue::StoreLocal { lvalue, value, .. }
        | InstructionValue::StoreContext { lvalue, value } => {
            visitor(lvalue.identifier.id);
            visitor(value.identifier.id);
        }
        InstructionValue::DeclareLocal { lvalue, .. }
        | InstructionValue::DeclareContext { lvalue } => {
            visitor(lvalue.identifier.id);
        }
        InstructionValue::Destructure { value, .. } => {
            visitor(value.identifier.id);
        }
        InstructionValue::BinaryExpression { left, right, .. } => {
            visitor(left.identifier.id);
            visitor(right.identifier.id);
        }
        InstructionValue::UnaryExpression { value, .. } => {
            visitor(value.identifier.id);
        }
        InstructionValue::PrefixUpdate { lvalue, .. }
        | InstructionValue::PostfixUpdate { lvalue, .. } => {
            visitor(lvalue.identifier.id);
        }
        InstructionValue::CallExpression { callee, args, .. }
        | InstructionValue::NewExpression { callee, args } => {
            visitor(callee.identifier.id);
            for arg in args {
                visitor(arg.identifier.id);
            }
        }
        InstructionValue::MethodCall { receiver, args, .. } => {
            visitor(receiver.identifier.id);
            for arg in args {
                visitor(arg.identifier.id);
            }
        }
        InstructionValue::PropertyLoad { object, .. }
        | InstructionValue::PropertyDelete { object, .. } => {
            visitor(object.identifier.id);
        }
        InstructionValue::PropertyStore { object, value, .. } => {
            visitor(object.identifier.id);
            visitor(value.identifier.id);
        }
        InstructionValue::ComputedLoad { object, property, .. }
        | InstructionValue::ComputedDelete { object, property } => {
            visitor(object.identifier.id);
            visitor(property.identifier.id);
        }
        InstructionValue::ComputedStore { object, property, value } => {
            visitor(object.identifier.id);
            visitor(property.identifier.id);
            visitor(value.identifier.id);
        }
        InstructionValue::ObjectExpression { properties } => {
            for prop in properties {
                visitor(prop.value.identifier.id);
                if let crate::hir::types::ObjectPropertyKey::Computed(p) = &prop.key {
                    visitor(p.identifier.id);
                }
            }
        }
        InstructionValue::ArrayExpression { elements } => {
            for elem in elements {
                match elem {
                    crate::hir::types::ArrayElement::Expression(p)
                    | crate::hir::types::ArrayElement::Spread(p) => visitor(p.identifier.id),
                    crate::hir::types::ArrayElement::Hole => {}
                }
            }
        }
        InstructionValue::JsxExpression { tag, props, children } => {
            visitor(tag.identifier.id);
            for attr in props {
                visitor(attr.value.identifier.id);
            }
            for child in children {
                visitor(child.identifier.id);
            }
        }
        InstructionValue::JsxFragment { children } => {
            for child in children {
                visitor(child.identifier.id);
            }
        }
        InstructionValue::TemplateLiteral { subexpressions, .. } => {
            for sub in subexpressions {
                visitor(sub.identifier.id);
            }
        }
        InstructionValue::TaggedTemplateExpression { tag, value } => {
            visitor(tag.identifier.id);
            for sub in &value.subexpressions {
                visitor(sub.identifier.id);
            }
        }
        InstructionValue::Await { value }
        | InstructionValue::StoreGlobal { value, .. }
        | InstructionValue::NextPropertyOf { value }
        | InstructionValue::TypeCastExpression { value, .. } => {
            visitor(value.identifier.id);
        }
        InstructionValue::GetIterator { collection } => {
            visitor(collection.identifier.id);
        }
        InstructionValue::IteratorNext { iterator, .. } => {
            visitor(iterator.identifier.id);
        }
        InstructionValue::FinishMemoize { decl, deps, .. } => {
            visitor(decl.identifier.id);
            for dep in deps {
                visitor(dep.identifier.id);
            }
        }
        InstructionValue::Primitive { .. }
        | InstructionValue::JSXText { .. }
        | InstructionValue::RegExpLiteral { .. }
        | InstructionValue::LoadGlobal { .. }
        | InstructionValue::StartMemoize { .. }
        | InstructionValue::UnsupportedNode { .. }
        | InstructionValue::FunctionExpression { .. }
        | InstructionValue::ObjectMethod { .. } => {}
    }
}

// Note: collect_operand_ids was replaced by visit_operand_ids (callback pattern)
// to avoid per-instruction Vec allocation. Other files still have their own copies.
