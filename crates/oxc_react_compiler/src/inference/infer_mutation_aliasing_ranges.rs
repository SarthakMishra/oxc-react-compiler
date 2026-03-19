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

use crate::hir::types::{AliasingEffect, BlockId, HIR, IdentifierId, InstructionId, MutableRange};

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

/// Propagate a mutation from `start` through the alias/capture graph via BFS.
///
/// For each reachable node, extends `mutableRange.end` to `end_instr`.
/// Upstream: `mutate()` in `InferMutationAliasingRanges.ts`.
fn mutate(
    graph: &AliasingGraph,
    ranges: &mut FxHashMap<IdentifierId, MutableRange>,
    start: IdentifierId,
    mutation_index: u32,
    end_instr: InstructionId,
    transitive: bool,
    start_kind: MutationKind,
    phi_ids: &FxHashSet<IdentifierId>,
) {
    let mut seen: FxHashMap<IdentifierId, MutationKind> = FxHashMap::default();
    let mut queue: VecDeque<WorkItem> = VecDeque::new();
    queue.push_back(WorkItem {
        place: start,
        transitive,
        direction: Direction::Backwards,
        kind: start_kind,
    });

    while let Some(item) = queue.pop_front() {
        // Dedup: skip if already visited with equal or stronger kind
        if let Some(&prev) = seen.get(&item.place)
            && prev >= item.kind
        {
            continue;
        }
        seen.insert(item.place, item.kind);

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
            queue.push_back(WorkItem {
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
            queue.push_back(WorkItem {
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
                queue.push_back(WorkItem {
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
                queue.push_back(WorkItem {
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
                queue.push_back(WorkItem {
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
pub fn infer_mutation_aliasing_ranges(hir: &mut HIR) {
    let mut graph = AliasingGraph::new();
    let mut phi_ids: FxHashSet<IdentifierId> = FxHashSet::default();
    let mut ranges: FxHashMap<IdentifierId, MutableRange> = FxHashMap::default();
    let mut mutations: Vec<PendingMutation> = Vec::new();
    let mut creation_map: FxHashMap<IdentifierId, InstructionId> = FxHashMap::default();

    // Pending phi operands for back-edges (predecessor block not yet visited)
    let mut pending_phis: FxHashMap<BlockId, Vec<(u32, IdentifierId, IdentifierId)>> =
        FxHashMap::default();

    let mut index: u32 = 0;
    let mut seen_blocks: FxHashSet<BlockId> = FxHashSet::default();

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
                        AliasingEffect::Mutate { value } => {
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
                        // DIVERGENCE: Apply should be resolved by analyse_functions
                        // but we don't have that pass yet, so skip gracefully.
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
    }

    // Phase 2: Process all mutations against the completed graph
    for m in &mutations {
        let end = InstructionId(m.instr_id.0 + 1);
        mutate(&graph, &mut ranges, m.place_id, m.index, end, m.transitive, m.kind, &phi_ids);
    }

    // Phase 3: Write mutable ranges back to HIR lvalue identifiers
    // Use creation_map for start, BFS-propagated ranges for end
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
}
