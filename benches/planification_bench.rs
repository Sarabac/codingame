use std::rc::Rc;

use codingame::common::StateBuilder;
use codingame::ligue1::ai::planifier;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;

fn planif_bench(c: &mut Criterion) {
    let state = Rc::new(StateBuilder::new_a_gauche_prot_a_a_droite().build());
    let mut group = c.benchmark_group("planif1");
    for size in 1..7usize {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| planifier(state.clone(), size));
        });
    }
    group.finish();
}

criterion_group!(benches, planif_bench);
criterion_main!(benches);
