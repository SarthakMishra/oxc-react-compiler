# Serena MCP Guidelines

Serena provides LSP-powered semantic code intelligence via MCP. It understands code structure through language servers, enabling precise symbol-level navigation and editing that surpasses plain text search.

## When to Prefer Serena Over Built-in Tools

### Symbol Navigation (prefer over Grep/Glob)

| Task                               | Use Serena                 | Instead of                                                          |
| ---------------------------------- | -------------------------- | ------------------------------------------------------------------- |
| Find a function/struct/type by name | `find_symbol`              | Grep for pattern                                                    |
| List exports/definitions in a file | `get_symbols_overview`     | Reading the entire file                                             |
| Find all usages of a symbol        | `find_referencing_symbols` | Grep for the name (misses renamed imports, catches false positives) |
| Find a file by name/glob           | `find_file`                | Glob (both work; Serena's respects .gitignore natively)             |

**Why**: Serena uses rust-analyzer, so it resolves trait implementations, generic bounds, re-exports, and type references correctly. Grep matches strings, not semantics.

### Symbol-Level Editing (prefer over Edit)

| Task                                       | Use Serena                                     | Instead of                                           |
| ------------------------------------------ | ---------------------------------------------- | ---------------------------------------------------- |
| Rename a function/variable across codebase | `rename_symbol`                                | Find-and-replace (misses some, hits false positives) |
| Replace a function/struct body              | `replace_symbol_body`                          | Edit with old_string/new_string                      |
| Insert code before/after a function        | `insert_before_symbol` / `insert_after_symbol` | Edit with surrounding context                        |

**Why**: Symbol-level edits are precise — they operate on the AST, not string matching. `rename_symbol` updates all references including imports, trait impls, and re-exports.

### When Built-in Tools Are Still Better

- **Simple text search across files** (e.g., searching for a string literal, URL, or config value) — use Grep
- **File discovery by pattern** (e.g., `*.rs`, `crates/**/*.rs`) — use Glob
- **Reading file contents** — use Read
- **Creating new files** — use Write
- **Simple line edits** where you know the exact text — Edit is fine
- **Git operations, shell commands, builds** — use Bash

## Key Tools Reference

### Navigation

- `find_symbol(name_path_pattern, relative_path?, ...)` — symbol search by name path pattern
  - `name_path_pattern`: simple name (`"my_fn"`), relative path (`"MyStruct/my_method"`), or absolute (`"/MyStruct/my_method"`)
  - `substring_matching`: partial name matching (e.g. `"get"` matches `"get_value"`, `"get_data"`)
  - `depth`: retrieve children (e.g. `depth=1` on a struct returns its fields/methods)
  - `include_body`: include the symbol's source code
  - `include_info`: include hover info (docstring + signature)
  - `include_kinds` / `exclude_kinds`: filter by LSP symbol kind
- `get_symbols_overview(relative_path, depth?)` — lists top-level symbols in a file (faster than reading + parsing). Use `depth=1` to also see struct fields and impl methods.
- `find_referencing_symbols(relative_path, name_path, ...)` — finds all symbols that reference the target, with code snippets around each reference
- `find_file(file_mask, relative_path)` — glob-like file search respecting .gitignore
- `search_for_pattern(substring_pattern, ...)` — regex search across project (respects .gitignore). Supports `relative_path`, `paths_include_glob`, `paths_exclude_glob`, `context_lines_before/after`.

### Editing

- `rename_symbol(relative_path, name_path, new_name)` — LSP-powered rename refactoring across all files
- `replace_symbol_body(relative_path, name_path, body)` — replaces the full definition of a symbol (body does NOT include preceding comments/imports)
- `insert_before_symbol(relative_path, name_path, body)` — inserts content before a symbol definition
- `insert_after_symbol(relative_path, name_path, body)` — inserts content after a symbol definition

### Memory (project-specific persistent context)

- `write_memory(name, content)` — store project knowledge for future sessions
- `read_memory(name)` — retrieve stored memory
- `list_memories()` — list all stored memories
- `edit_memory(name, needle, repl, mode)` — replace content in a memory (`mode`: "literal" or "regex")
- `delete_memory(name)` — delete a memory
- `rename_memory(old_name, new_name)` — rename/move a memory

### Project

- `onboarding()` — identifies project structure (run once per project setup)
- `check_onboarding_performed()` — check if onboarding was already done
- `open_dashboard()` — opens the Serena web dashboard

## Usage Patterns

### Investigating a function's impact before changing it

```
1. find_symbol("infer_types") → locates the definition
2. find_referencing_symbols(file, "infer_types") → shows all callers + code snippets
3. get_symbols_overview(caller_file) → understand caller context
4. Make informed edit
```

### Safe rename refactoring

```
1. find_referencing_symbols(file, "old_name") → verify scope of change
2. rename_symbol(file, "old_name", "new_name") → LSP handles all references
```

### Understanding a module

```
1. get_symbols_overview("crates/oxc_react_compiler/src/hir/types.rs") → see all exports at a glance
2. find_referencing_symbols for key types → understand dependency graph
```

### Exploring a struct/enum

```
1. find_symbol("InstructionValue", depth=1) → see all variants without reading the file
2. find_symbol("InstructionValue/CallExpression", include_body=True) → read just the variant you need
```

## Important Notes

- The `claude-code` context is active, which disables tools that duplicate Claude Code built-ins (file creation, shell commands, string replacement, etc.)
- Serena operates on rust-analyzer — it understands `.rs` files natively, and `.ts`/`.tsx`/`.js` files for the NAPI/Vite plugin layer
- After external edits (e.g., git operations), the language server may need a moment to re-index
- Memory files are stored in `.serena/memories/` (gitignored) and persist across sessions
- Run `onboarding` once when first setting up to let Serena understand the project structure
