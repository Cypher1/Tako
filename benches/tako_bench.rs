#![allow(unused)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use takolib::cli_options::Options;
use takolib::database::Compiler;
use takolib::database::DB;
use takolib::parser::parse_string;
use takolib::type_checker::infer;
use takolib::primitives::Val::Variable;

use std::sync::Arc;

pub fn criterion_benchmark(c: &mut Criterion) {
    let module = vec![];

    c.bench_function("microbench_type_of_i32", |b| {
        let code = Arc::new("12".to_string());
        let mut db = DB::default();
        db.set_options(Options::default());
        let prog = black_box(parse_string(&db, &module, &code).expect("should parse"));
        let env = Variable("test_program".to_string()); // TODO: Track the type env
        b.iter(|| infer(&db, &prog, &env));
    });

    c.bench_function("microbench_parse_and_type_of_i32_pre_cache", |b| {
        let code = Arc::new("12".to_string());
        let mut db = DB::default();
        db.set_options(Options::default());
        let prog = parse_string(&db, &module, &code).expect("should parse");
        let env = Variable("test_program".to_string()); // TODO: Track the type env
        infer(&db, &prog, &env);
        b.iter(|| {
            let prog = black_box(parse_string(&db, &module, &code).expect("should parse"));
            infer(&db, &prog, &env)
        });
    });

    c.bench_function("microbench_parse_and_type_of_i32", |b| {
        let code = Arc::new("12".to_string());
        let mut db = DB::default();
        db.set_options(Options::default());
        let prog = black_box(parse_string(&db, &module, &code).expect("should parse"));
        let env = Variable("test_program".to_string()); // TODO: Track the type env
        b.iter(|| infer(&db, &prog, &env));
    });

    /* c.bench_function(
        "microbench_type_of_plus_expr",
        |b| {
            let mut db = DB::default();
            db.set_options(Options::default());
            let prog = black_box(parse_string(&mut db, "12+32".to_string()));
            b.iter(|| infer(&db, &prog, &env));
        }
    );*/
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
