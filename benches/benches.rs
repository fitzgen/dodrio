#![cfg(all(
    feature = "xxx-unstable-internal-use-only",
    not(target_arch = "wasm32")
))]

use criterion::{
    black_box, criterion_group, criterion_main, Benchmark, Criterion, ParameterizedBenchmark,
    Throughput,
};
use dodrio::{
    builder::*,
    bumpalo::{self, Bump},
    Node, Render, RenderContext, Vdom,
};

/// The simplest thing we can render: `<div/>`.
struct Empty;
impl Render for Empty {
    fn render<'bump>(&self, cx: &mut RenderContext<'bump>) -> Node<'bump> {
        div(&cx).finish()
    }
}

/// Render a list that is `self.0` items long, has attributes and listeners.
struct SimpleList(usize);
impl Render for SimpleList {
    fn render<'bump>(&self, cx: &mut RenderContext<'bump>) -> Node<'bump> {
        let mut children = bumpalo::collections::Vec::with_capacity_in(self.0, cx.bump);
        children.extend((0..self.0).map(|_| {
            li(&cx)
                .attr("class", "my-list-item")
                .on("click", |_root, _vdom, _event| {
                    panic!("no one should call this")
                })
                .children([text("a list item")])
                .finish()
        }));
        ol(&cx).attr("id", "my-list").children(children).finish()
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "render",
        Benchmark::new("empty", {
            let mut bump = Bump::new();
            move |b| {
                bump.reset();
                let mut cx = RenderContext::new(&bump);
                b.iter(|| {
                    black_box(Empty.render(&mut cx));
                })
            }
        }),
    );

    c.bench(
        "render",
        ParameterizedBenchmark::new(
            "list",
            {
                let mut bump = Bump::new();
                move |b, n| {
                    bump.reset();
                    let mut cx = RenderContext::new(&bump);
                    b.iter(|| {
                        black_box(SimpleList(*n).render(&mut cx));
                    })
                }
            },
            vec![
                // TODO: Only test one `n` value until
                // https://github.com/bheisler/criterion.rs/issues/269 is fixed.
                //
                // 100,
                // 1_000,
                10_000,
            ],
        )
        .throughput(|n| Throughput::Elements(*n as u32)),
    );

    c.bench(
        "render-and-diff",
        ParameterizedBenchmark::new(
            "same-list",
            |b, &n| {
                let vdom = Vdom::new(&(), SimpleList(n));
                b.iter(|| {
                    vdom.immediately_render_and_diff(SimpleList(n));
                    black_box(&vdom);
                })
            },
            vec![
                // TODO: Only test one `n` value until
                // https://github.com/bheisler/criterion.rs/issues/269 is fixed.
                //
                // 100,
                // 1_000,
                10_000,
            ],
        )
        .with_function("empty-to-full-list-to-empty", |b, &n| {
            b.iter(|| {
                let vdom = Vdom::new(&(), Empty);
                vdom.immediately_render_and_diff(SimpleList(n));
                black_box(&vdom);
                vdom.immediately_render_and_diff(Empty);
                black_box(&vdom);
            });
        })
        .with_function("append-one-and-remove-one", |b, &n| {
            let vdom = Vdom::new(&(), SimpleList(n));
            b.iter(|| {
                vdom.immediately_render_and_diff(SimpleList(n + 1));
                black_box(&vdom);
                vdom.immediately_render_and_diff(SimpleList(n));
                black_box(&vdom);
            })
        })
        .throughput(|n| Throughput::Elements(*n as u32)),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
