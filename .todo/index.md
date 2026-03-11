# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-11

---

---

## Priority 3: Tier 2 Lint Rules

- [x] check_hooks_tier2: full Rules of Hooks with CFG analysis — wired through run_full_pipeline + DiagnosticKind filtering
- [x] check_immutability: mutation of frozen values — wired through run_full_pipeline + DiagnosticKind::ImmutabilityViolation
- [x] check_preserve_manual_memoization — wired through run_full_pipeline (pass 61 now reachable)
- [x] check_memo_dependencies: exhaustive useMemo/useCallback deps — enabled validate_exhaustive_memo_dependencies in lint config
- [x] check_exhaustive_effect_deps: exhaustive useEffect deps — enabled validate_exhaustive_effect_dependencies in lint config

---

---

## Priority 5: End-of-Project Cleanup

- [x] Fix all clippy warnings — auto-fixed 258 mechanical issues, added crate-level `#[allow]` for style lints. Zero warnings across workspace.

---

## Active Work

_(Nothing in progress)_

---

## Blocked

_(Nothing blocked)_
