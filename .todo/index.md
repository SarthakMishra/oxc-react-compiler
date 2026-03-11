# OXC React Compiler -- Backlog Index

> Comprehensive backlog for porting babel-plugin-react-compiler to Rust/OXC.
> Items are ordered by dependency: nothing should be blocked by an item below it.

Last updated: 2026-03-11

---

## Priority 1: Upstream Conformance

- [ ] Port upstream fixture inputs into test suite — [conformance.md](conformance.md)#gap-1-port-upstream-fixture-inputs
- [ ] Run upstream Babel plugin as reference oracle — [conformance.md](conformance.md)#gap-2-run-upstream-babel-plugin-as-reference-oracle
- [ ] Build differential comparison harness — [conformance.md](conformance.md)#gap-3-differential-comparison-harness
- [ ] Add behavioral equivalence normalization — [conformance.md](conformance.md)#gap-4-behavioral-equivalence-normalization
---

## Priority 2: Source Map Support

- [ ] Expose source map from compile_program — [source-maps.md](source-maps.md)#gap-1-expose-source-map-from-compile_program
- [ ] Whole-file source map composition — [source-maps.md](source-maps.md)#gap-4-whole-file-source-map-composition
- [ ] Pass source map through NAPI binding — [source-maps.md](source-maps.md)#gap-2-pass-source-map-through-napi-binding
- [ ] Wire source map in Vite plugin — [source-maps.md](source-maps.md)#gap-3-wire-source-map-in-vite-plugin

---

## Priority 3: Tier 2 Lint Rules

- [ ] Structured error categories for lint filtering — [tier2-lint.md](tier2-lint.md)#gap-6-structured-error-categories-for-lint-filtering
- [ ] check_hooks_tier2: full Rules of Hooks with CFG analysis — [tier2-lint.md](tier2-lint.md)#gap-1-check_hooks_tier2----full-rules-of-hooks-with-cfg-analysis
- [ ] check_immutability: mutation of frozen values — [tier2-lint.md](tier2-lint.md)#gap-2-check_immutability----mutation-of-frozen-values
- [ ] check_preserve_manual_memoization — [tier2-lint.md](tier2-lint.md)#gap-3-check_preserve_manual_memoization
- [ ] check_memo_dependencies: exhaustive useMemo/useCallback deps — [tier2-lint.md](tier2-lint.md)#gap-4-check_memo_dependencies----exhaustive-usememousecallback-deps
- [ ] check_exhaustive_effect_deps: exhaustive useEffect deps — [tier2-lint.md](tier2-lint.md)#gap-5-check_exhaustive_effect_deps----exhaustive-useeffect-deps

---

## Priority 4: Documentation

- [ ] Vite plugin usage guide — [docs.md](docs.md)#gap-1-vite-plugin-usage-guide
- [ ] Lint rules documentation — [docs.md](docs.md)#gap-2-lint-rules-documentation
- [ ] Configuration reference — [docs.md](docs.md)#gap-3-configuration-reference
- [ ] Known limitations section — [docs.md](docs.md)#gap-4-known-limitations-section

---

## Priority 5: End-of-Project Cleanup

- [ ] Fix all clippy warnings (`cargo clippy --all-targets --all-features`) — iteratively fix real issues (unused variables, unused imports, type casts, missing docs, etc.) and suppress false positives with targeted `#[allow(...)]` attributes where appropriate. This is strictly last-priority cleanup work.

---

## Active Work

_(Nothing in progress)_

---

## Blocked

_(Nothing blocked)_
