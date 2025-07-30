use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use wirerust::FieldType;
use wirerust::*;

fn bench_parse_compile_execute(c: &mut Criterion) {
    let schema = Arc::new(
        FilterSchemaBuilder::new()
            .field("foo", FieldType::Int)
            .field("bar", FieldType::Bytes)
            .build(),
    );
    let engine = WirerustEngine::new((*schema).clone());
    let expr_str = "foo == 42 && bar == \"baz\"";
    let ctx = FilterContextBuilder::new(&schema)
        .set_int("foo", 42)
        .unwrap()
        .set_bytes("bar", b"baz")
        .unwrap()
        .build();

    c.bench_function("parse", |b| {
        b.iter(|| {
            let _ = engine.parse_filter(black_box(expr_str));
        })
    });
    c.bench_function("parse_and_compile", |b| {
        b.iter(|| {
            let _ = engine.parse_and_compile(black_box(expr_str));
        })
    });
    let compiled = engine.parse_and_compile(expr_str).unwrap();
    c.bench_function("execute", |b| {
        b.iter(|| {
            let _ = engine.execute(&compiled, &ctx);
        })
    });
}

criterion_group!(benches, bench_parse_compile_execute);
criterion_main!(benches);
