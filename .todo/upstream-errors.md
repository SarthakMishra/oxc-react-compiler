# Upstream Errors We Should Match (126 fixtures)

Upstream rejects with an error, we either compile or produce the wrong error.
This is the "we compile, they don't" category -- 126 fixtures at 415/1717.

Note: this number increased from 95 to 126 as we fixed bail-outs that were
previously masking over-compilation (we now compile more fixtures, some of
which upstream rejects).

## Error Categories

### Upstream Internal Errors (Invariant/Todo) -- partly matched

**Matched (via `validate_no_unsupported_nodes`):**
- YieldExpression, ClassExpression, getter/setter syntax, new.target, for-await-of

**Still unmatched (known upstream limitations, low priority):**
- "Invariant: [InferMutationAliasingEffects] Expected..." (3)
- "Invariant: [Codegen] Internal error..." (2)
- "Todo: [PruneHoistedContexts]..." (2)
- "Todo: Support duplicate fbt tags..." (2)
- "Invariant: Expected temporaries to be promoted..." (1)
- "Todo: (BuildHIR::lowerExpression) Handle UpdateExpression..." (1)
- "Todo: (BuildHIR::lowerStatement) Handle var kinds..." (1)

These are known-skips representing upstream limitations.

### Validation Errors We Should Match

From the DIVERGED list (7 unexpected divergences):
- `error.bug-invariant-expected-consistent-destructuring.js`
- `error.invalid-jsx-captures-context-variable.js`
- `error.invalid-setState-in-useMemo-indirect-useCallback.js`
- `error.repro-preserve-memoization-inner-destructured-value-mistaken-as-dependency-later-mutation.js`
- `error.todo-for-loop-with-context-variable-iterator.js`
- `error.todo-missing-source-locations.js`
- `exhaustive-deps/error.invalid-exhaustive-effect-deps-missing-only.js`

### Over-Compilation in Infer Mode

The remaining ~119 fixtures where we compile but upstream doesn't are
likely a mix of:
1. Functions upstream skips in `compilationMode:"infer"` (non-component, non-hook)
2. Functions where upstream emits a validation error we haven't implemented
3. Cases where our component/hook detection heuristics differ from upstream

**Fix strategy:**
1. Check if `@compilationMode` directive parsing is complete
2. Verify component/hook detection matches upstream heuristics
3. Match individual upstream validation errors case-by-case

**Key files:**
- `crates/oxc_react_compiler/src/entrypoint/program.rs`
- Various validation passes in `crates/oxc_react_compiler/src/validation/`
