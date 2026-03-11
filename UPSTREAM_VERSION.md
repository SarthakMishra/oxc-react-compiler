# Upstream React Compiler Version

This file tracks the upstream `babel-plugin-react-compiler` commit that this
port is based on.

## Current Baseline

- **Repository:** https://github.com/facebook/react
- **Package:** `compiler/packages/babel-plugin-react-compiler`
- **Commit:** `HEAD` (initial port, not pinned to specific commit yet)
- **Date:** 2026-03-11

## Sync Status

This port covers the full 62-pass compilation pipeline, including:
- HIR types and core data structures
- BuildHIR (OXC AST -> HIR lowering)
- SSA conversion (EnterSSA, EliminateRedundantPhi)
- Optimization passes (ConstantPropagation, DCE, MergeBlocks, etc.)
- Type inference and mutation/aliasing analysis
- Reactive scope inference, alignment, and merging
- ReactiveFunction construction and pruning
- Code generation with useMemoCache

## How to Update

1. Check the upstream commit log for changes to the compiler
2. Compare upstream TypeScript types with our Rust HIR types
3. Update pass implementations to match upstream behavior
4. Run comparison tests to verify output matches
