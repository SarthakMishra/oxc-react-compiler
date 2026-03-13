# Flow Fixtures

> **Priority**: P5 (38 fixtures, lowest tractability)
> **Impact**: 38 fixtures use Flow type annotations that OXC parser cannot handle

## Problem Statement

38 upstream fixtures use `@flow` type annotations. The OXC parser only supports
TypeScript and JavaScript, not Flow. These fixtures fail at the parse stage.

### Gap 1: Strategy

**Options:**
1. **Skip permanently** -- Flow is being deprecated in the React ecosystem. Mark these as known skips. Cost: 0 effort, lose 38 fixtures forever.
2. **Flow-to-TS preprocessing** -- Use a tool like `flow-to-ts` to convert Flow annotations to TypeScript before parsing. Cost: moderate (need to add a preprocessing step), gain 38 fixtures.
3. **Strip Flow types** -- Use a Flow type stripper (like `flow-remove-types`) to remove annotations before parsing. Cost: low-moderate, but may lose type information needed for compilation.

**Recommendation:** Option 1 (skip). Flow is actively being replaced by TypeScript in the React ecosystem. The 38 fixtures represent 2.2% of the total suite -- not worth the infrastructure investment.

**Depends on:** None
