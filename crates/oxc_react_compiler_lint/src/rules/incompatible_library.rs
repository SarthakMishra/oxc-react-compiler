#![allow(dead_code)]
//! # incompatible-library
//!
//! Detects imports from libraries known to be incompatible with the React
//! compiler. These libraries may rely on mutation patterns or other behaviors
//! that break compiler assumptions.

use oxc_ast::ast::*;
use oxc_ast_visit::{Visit, walk};
use oxc_diagnostics::OxcDiagnostic;

/// Libraries known to be incompatible with the React compiler.
const BLOCKLISTED_LIBRARIES: &[&str] =
    &["mobx", "mobx-react", "mobx-react-lite", "valtio", "valtio/utils", "immer"];

/// Check for imports from blocklisted libraries.
pub fn check_incompatible_library<'a>(program: &Program<'a>) -> Vec<OxcDiagnostic> {
    let mut visitor = IncompatibleLibraryVisitor { diagnostics: Vec::new() };
    visitor.visit_program(program);
    visitor.diagnostics
}

struct IncompatibleLibraryVisitor {
    diagnostics: Vec<OxcDiagnostic>,
}

impl<'a> Visit<'a> for IncompatibleLibraryVisitor {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        let source = it.source.value.as_str();
        for blocklisted in BLOCKLISTED_LIBRARIES {
            if source == *blocklisted || source.starts_with(&format!("{}/", blocklisted)) {
                self.diagnostics.push(
                    OxcDiagnostic::warn(format!(
                        "Import from \"{}\" is incompatible with the React compiler. \
                         This library uses patterns that may break compiler optimizations.",
                        source
                    ))
                    .with_label(it.span),
                );
                break;
            }
        }

        walk::walk_import_declaration(self, it);
    }
}
