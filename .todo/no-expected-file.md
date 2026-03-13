# No Expected File -- Missing Upstream Outputs

> **Priority**: P4 (261 fixtures, low tractability)
> **Impact**: 261 fixtures have no Babel expected output to compare against

## Problem Statement

These fixtures exist in the upstream test suite but have no corresponding
`__expected__` output file. Without a reference output, we cannot determine
whether our compiler's output is correct.

### Gap 1: Generate Expected Outputs

**Upstream:** Run `babel-plugin-react-compiler` on each fixture to generate expected outputs
**Current state:** The conformance test infrastructure skips these fixtures
**What's needed:**
- Run the upstream Babel compiler (v1.0.0) on all 261 fixtures
- Save the outputs as expected files
- Add them to the conformance comparison
- Some fixtures may intentionally have no output (e.g., they test error-only behavior)
**Depends on:** None

## Notes

- These may represent newer fixtures added to upstream after our initial download
- Some may be fixtures that test compiler internals (no user-facing output)
- Priority is low because we cannot measure improvement without references
