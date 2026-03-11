# oxc-react-compiler Agent Guidelines

## Tech Stack

- **Language**: Rust
- **Parser/AST**: OXC (`oxc_parser`, `oxc_ast`, `oxc_semantic`, `oxc_span`)
- **Build**: Cargo workspace
- **NAPI Bindings**: `napi-rs` (for Node.js/Vite integration)
- **Plugin Layer**: TypeScript (Vite plugin)
- **Package Manager**: pnpm (for TypeScript/NAPI layer)

## Rust Conventions

- Use `cargo check` / `cargo clippy` for linting
- Use `cargo test` for running tests
- Use `cargo fmt` / `rustfmt` for formatting
- Use `FxHashMap` / `FxHashSet` from `rustc-hash` for internal maps (not `std::HashMap`)
- Use `IndexMap` from `indexmap` for ordered maps (block ordering in CFG)
- Use `oxc_allocator::Allocator` for arena-allocated AST nodes
- Prefer `&str` over `String` where possible; use `oxc_span::Atom` for interned strings

## Serena (LSP-Powered Code Intelligence)

Serena MCP provides semantic code navigation and editing via rust-analyzer and the TypeScript language server. **Prefer Serena tools over built-in Grep/Glob/Edit for symbol-level operations.**

### Prefer Serena for:

- **Finding symbols**: `find_symbol` instead of Grep — resolves trait impls, re-exports, generic bounds
- **Listing file exports**: `get_symbols_overview` instead of reading the whole file
- **Finding all usages**: `find_referencing_symbols` instead of Grep — no false positives from string matching
- **Renaming across codebase**: `rename_symbol` instead of find-and-replace — updates all references including imports
- **Replacing function/struct bodies**: `replace_symbol_body` instead of Edit — operates on AST, not string matching

### Keep using built-in tools for:

- Text/string/config searches → Grep
- File discovery by pattern → Glob
- Reading file contents → Read
- Creating new files → Write
- Simple known-text edits → Edit
- Git, shell, builds → Bash

See `.claude/rules/serena.md` for detailed tool reference and usage patterns.

## Crate Structure

```
oxc-react-compiler/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── oxc_react_compiler/             # Core compiler crate
│   │   └── src/
│   │       ├── hir/                    # HIR types, Environment, BuildHIR
│   │       ├── ssa/                    # SSA conversion
│   │       ├── optimization/           # Optimization passes
│   │       ├── inference/              # Type & mutation inference
│   │       ├── reactive_scopes/        # Scope inference, alignment, codegen
│   │       ├── validation/             # Validation passes
│   │       ├── entrypoint/             # Pipeline, program, options
│   │       ├── utils/                  # DisjointSet, OrderedMap
│   │       └── error.rs               # CompilerError
│   └── oxc_react_compiler_lint/        # Oxlint rule implementations
│       └── src/rules/
├── napi/react-compiler/                # NAPI-RS Node.js bindings + Vite plugin
├── justfile
└── pnpm-workspace.yaml
```

## Upstream Compatibility

This project is a 1:1 port of [babel-plugin-react-compiler](https://github.com/facebook/react/tree/main/compiler/packages/babel-plugin-react-compiler). Key conventions:

- **1:1 file mapping**: Each upstream TypeScript file maps to a Rust module (see REQUIREMENTS.md Appendix A)
- **Naming**: `snake_case` functions for upstream `camelCase`, `PascalCase` types preserved
- **Algorithm fidelity**: Core passes must be algorithmically identical to upstream
- **Divergences**: Mark any intentional divergence with `// DIVERGENCE:` comments
- **Boundary layers**: Only `BuildHIR` (OXC AST → HIR) and `Codegen` (ReactiveFunction → OXC AST) are OXC-specific; everything else is a direct port

## Skills

Detailed conventions are organized as skills in `.claude/skills/`. Each skill is loaded on-demand when relevant to the task.

| Skill        | When to Use                                                                    |
| ------------ | ------------------------------------------------------------------------------ |
| `commit`     | Conventional commit message formatting with proper type, scope, description    |
| `journal`    | Recording completed work as phase entries in `.journal/` with file tables       |
| `taskmaster` | Backlog analysis, task prioritization, feature planning, `.todo/` management    |

## Agents

Specialized subagents that handle complex, multi-step tasks autonomously. Delegate to these when the task matches their scope.

| Agent         | When to Delegate                                                                                                             |
| ------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| `plan`        | Designing implementation plans — researches the codebase via explore sub-agent, returns structured plan with files and steps  |
| `explore`     | Fast codebase exploration using Serena's LSP tools — finding symbols, tracing references, understanding module structure      |
| `commit`      | Creating git commits with conventional commit messages, analyzing diffs, staging files                                        |
| `code-review` | Reviewing diffs, preparing PRs, auditing changes for correctness, upstream fidelity, performance, and safety                  |
| `taskmaster`  | Backlog analysis, prioritization, feature planning, and `.todo/` reconciliation after implementation                          |
| `journal`     | Recording implementation sessions in `.journal/` with phase entries, file tables, and upstream references                     |
| `deep-work`   | Autonomous end-to-end sessions: picks a task, plans, implements, reviews, fixes, journals, reconciles backlog, and commits    |

### Common Workflows

**Plan a compiler pass:**
`plan` (research + design) → review plan → implement → `code-review` → fix issues → `commit`

**Explore before implementing:**
`explore` (understand existing code + upstream reference) → implement → `code-review` → `commit`

**Pre-merge audit:**
`code-review` (diff audit) → fix findings → `commit`

**Autonomous deep-work session:**
`deep-work` (runs all phases end-to-end: taskmaster → plan → implement → code-review → fix → journal → todo reconciliation → commit)

**Decide what to build next:**
`taskmaster` (analyzes `.todo/` backlog, `REQUIREMENTS.md` pipeline, and `.journal/` history → ONE recommendation)

## Critical Rules (Always Active)

- **Match upstream behavior** — the 50+ core passes must produce identical results to the TypeScript compiler
- **No unnecessary `unwrap()`** — handle errors properly via `CompilerError` or `Result`
- **Arena allocate AST nodes** — use `oxc_allocator` for any OXC AST nodes created during codegen
- **No secrets in code** — API tokens come from env vars
- **Performance matters** — the compiler runs on every file transform; avoid unnecessary allocations in hot paths
- **Use `FxHash*` collections** — not `std::collections::Hash*` for internal compiler maps
- **Parameterize all external input** — no string interpolation for user-provided values
- **Document divergences** — any intentional difference from upstream gets a `// DIVERGENCE:` comment

## Architecture Reference

The full architecture, 62-pass pipeline ordering, HIR data structures, and upstream file mapping are documented in `REQUIREMENTS.md`.
