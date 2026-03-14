# Upstream Errors We Should Match (39 fixtures)

Upstream rejects with an error, we either compile or produce the wrong error.

## Error Categories

### Upstream Internal Errors (Invariant/Todo) -- 16 fixtures, SKIP

These are upstream compiler bugs/limitations, not validation errors:
- "Invariant: [InferMutationAliasingEffects] Expected..." (3)
- "Invariant: [Codegen] Internal error..." (2)
- "Todo: [PruneHoistedContexts]..." (2)
- "Todo: Support duplicate fbt tags..." (2)
- "Todo: (BuildHIR::lowerExpression) Handle YieldExpression..." (1)
- "Invariant: Expected temporaries to be promoted..." (1)
- "Todo: (BuildHIR::lowerStatement) Handle for-await..." (1)
- "Todo: (BuildHIR::lowerExpression) Handle UpdateExpression..." (1)
- "Todo: (BuildHIR::lowerStatement) Handle var kinds..." (1)
- "Todo: (BuildHIR::lowerExpression) Handle MetaProperty..." (1)
- "Todo: (BuildHIR::lowerExpression) Handle get function..." (1)

**Action:** Mark these 16 as known-skips. They represent upstream
limitations that we should NOT match.

### Validation Errors We Should Match -- 23 fixtures

These are real validation errors where upstream correctly rejects:

- "This value cannot be modified" (2) -- frozen mutation
- "Cannot modify local variables after render" (2) -- locals reassigned
- "Invalid type configuration for module" (2) -- type provider
- "Compilation Skipped: Existing memoization" (3) -- preserve-memo
- "Cannot access variable before declared" (1) -- TDZ/hoisting
- "This value cannot be modified (component props)" (1)
- "Support spread syntax for hook arguments" (1) -- hook spread
- "Support functions with unreachable code" (1) -- unreachable
- "Const declaration cannot be referenced before init" (1)
- "Support local variables named `fbt`" (1)
- "fbt tags should be module-level imports" (1)
- "Dynamic gating directive invalid" (1)
- "Unexpected empty block with goto" (1)
- "BuildHIR::lowerStatement Handle ThrowStatement" (1)
- "EnterSSA: Expected identifier" (1)
- "Expected variable declaration" (1)

**Fix strategy:** Each requires individual analysis. The 3 preserve-memo
and 2 type-provider fixtures are the most likely to yield gains.
The Invariant/Todo errors should probably be matched with our own
bail-out diagnostics rather than trying to reproduce the exact error.

**Key files:**
- Various validation passes in `crates/oxc_react_compiler/src/validation/`
- `crates/oxc_react_compiler/src/entrypoint/pipeline.rs`
