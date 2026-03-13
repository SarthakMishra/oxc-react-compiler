//! Benchmarks for the React Compiler pipeline.
//!
//! Run with: cargo bench -p oxc_react_compiler

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxc_react_compiler::{PluginOptions, compile_program};
use std::path::Path;

/// Load all fixture files from the tests/fixtures directory.
fn load_fixtures() -> Vec<(String, String)> {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut fixtures = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension()
                && (ext == "tsx" || ext == "ts" || ext == "jsx" || ext == "js")
            {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                let source = std::fs::read_to_string(&path).unwrap();
                fixtures.push((name, source));
            }
        }
    }

    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    fixtures
}

fn bench_compile_individual(c: &mut Criterion) {
    let fixtures = load_fixtures();
    let options = PluginOptions::default();

    let mut group = c.benchmark_group("compile_individual");
    for (name, source) in &fixtures {
        group.bench_function(name, |b| {
            b.iter(|| compile_program(black_box(source), black_box(name), black_box(&options)));
        });
    }
    group.finish();
}

fn bench_compile_all(c: &mut Criterion) {
    let fixtures = load_fixtures();
    let options = PluginOptions::default();

    c.bench_function("compile_all_fixtures", |b| {
        b.iter(|| {
            for (name, source) in &fixtures {
                let _ = compile_program(black_box(source), black_box(name), black_box(&options));
            }
        });
    });
}

fn bench_parse_only(c: &mut Criterion) {
    let fixtures = load_fixtures();

    c.bench_function("parse_only_all_fixtures", |b| {
        b.iter(|| {
            for (name, source) in &fixtures {
                let allocator = oxc_allocator::Allocator::default();
                let source_type = oxc_span::SourceType::from_path(name).unwrap_or_default();
                let _ = oxc_parser::Parser::new(&allocator, black_box(source), source_type).parse();
            }
        });
    });
}

criterion_group!(benches, bench_compile_individual, bench_compile_all, bench_parse_only);
criterion_main!(benches);
