//! Benchmarks for React Compiler lint rules.
//!
//! Run with: cargo bench -p oxc_react_compiler_lint

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_react_compiler_lint::{run_all_lint_rules, run_lint_rules};
use oxc_span::SourceType;

const SIMPLE_COMPONENT: &str = r#"
function Greeting({ name }) {
    return <div>Hello {name}</div>;
}
"#;

const COMPONENT_WITH_HOOKS: &str = r#"
function Counter() {
    const [count, setCount] = useState(0);
    const doubled = useMemo(() => count * 2, [count]);
    useEffect(() => {
        document.title = `Count: ${count}`;
    }, [count]);
    return (
        <div>
            <p>{doubled}</p>
            <button onClick={() => setCount(count + 1)}>+</button>
        </div>
    );
}
"#;

const COMPLEX_COMPONENT: &str = r#"
import { observable } from "mobx";

function Dashboard({ user, items, filter }) {
    const [search, setSearch] = useState("");
    const [page, setPage] = useState(0);

    const filtered = useMemo(
        () => items.filter(item => item.name.includes(search) && item.type === filter),
        [items, search, filter]
    );

    useEffect(() => {
        console.log("Filtered items:", filtered.length);
    }, [filtered]);

    const handleSearch = useCallback((e) => {
        setSearch(e.target.value);
        setPage(0);
    }, []);

    if (!user) {
        useState(0);
    }

    try {
        return (
            <div>
                <input value={search} onChange={handleSearch} />
                {filtered.slice(page * 10, (page + 1) * 10).map(item => (
                    <div key={item.id}>{item.name}</div>
                ))}
            </div>
        );
    } catch (e) {
        return <div>Error</div>;
    }
}
"#;

fn bench_tier1_lint(c: &mut Criterion) {
    let mut group = c.benchmark_group("tier1_lint");

    for (name, source) in [
        ("simple", SIMPLE_COMPONENT),
        ("hooks", COMPONENT_WITH_HOOKS),
        ("complex", COMPLEX_COMPONENT),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| {
                let allocator = Allocator::default();
                let source_type = SourceType::tsx();
                let ret = Parser::new(&allocator, black_box(source), source_type).parse();
                run_lint_rules(&ret.program)
            });
        });
    }
    group.finish();
}

fn bench_all_lint(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_lint");

    for (name, source) in [
        ("simple", SIMPLE_COMPONENT),
        ("hooks", COMPONENT_WITH_HOOKS),
        ("complex", COMPLEX_COMPONENT),
    ] {
        group.bench_function(name, |b| {
            b.iter(|| {
                let allocator = Allocator::default();
                let source_type = SourceType::tsx();
                let ret = Parser::new(&allocator, black_box(source), source_type).parse();
                run_all_lint_rules(&ret.program)
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_tier1_lint, bench_all_lint);
criterion_main!(benches);
