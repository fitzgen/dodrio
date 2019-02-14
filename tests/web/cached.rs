use super::{assert_rendered, create_element, RenderFn};
use dodrio::{builder::*, bumpalo::Bump, Cached, Node, Render, Vdom};
use futures::prelude::*;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::*;
use wasm_bindgen_test::*;

pub struct CountRenders {
    render_count: Cell<usize>,
}

impl CountRenders {
    pub fn new() -> CountRenders {
        CountRenders {
            render_count: Cell::new(0),
        }
    }
}

impl Render for CountRenders {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        let count = self.render_count.get() + 1;
        self.render_count.set(count);

        let s = bumpalo::format!(in bump, "{}", count);
        text(s.into_bump_str())
    }
}

#[wasm_bindgen_test(async)]
fn uses_cached_render() -> impl Future<Item = (), Error = JsValue> {
    use dodrio::builder::*;

    let cached = Cached::new(CountRenders::new());

    let container0 = create_element("div");
    let container1 = container0.clone();
    let container2 = container0.clone();
    let container3 = container0.clone();
    let container4 = container0.clone();

    let vdom0 = Rc::new(Vdom::new(&container0, cached));
    let vdom1 = vdom0.clone();
    let vdom2 = vdom0.clone();
    let vdom3 = vdom0.clone();
    let vdom4 = vdom0.clone();
    let vdom5 = vdom0.clone();
    let vdom6 = vdom0.clone();
    let vdom7 = vdom0.clone();

    vdom0
        .weak()
        // We render, populate the cache, and get "1".
        .render()
        .and_then(move |_| {
            vdom0.weak().with_component(move |comp| {
                let comp = comp.unwrap_mut::<Cached<CountRenders>>();
                assert_eq!(comp.render_count.get(), 1);
                assert_rendered(&container1, &RenderFn(|_| text("1")));
            })
        })
        // We re-render, re-use the cached node, and get "1" again.
        .and_then(move |_| vdom1.weak().render())
        .and_then(move |_| {
            vdom2.weak().with_component(move |comp| {
                let comp = comp.unwrap_mut::<Cached<CountRenders>>();
                assert_eq!(comp.render_count.get(), 1);
                assert_rendered(&container2, &RenderFn(|_| text("1")));
            })
        })
        // We invalidate the cache, re-render, re-populate the cache, and should
        // now have "2".
        .and_then(move |_| {
            vdom3.weak().with_component(move |comp| {
                let comp = comp.unwrap_mut::<Cached<CountRenders>>();
                Cached::invalidate(comp);
            })
        })
        .and_then(move |_| vdom4.weak().render())
        .and_then(move |_| {
            vdom5.weak().with_component(move |comp| {
                let comp = comp.unwrap_mut::<Cached<CountRenders>>();
                assert_eq!(comp.render_count.get(), 2);
                assert_rendered(&container3, &RenderFn(|_| text("2")));
            })
        })
        // We re-render, re-use the cached node, and get "2" again.
        .and_then(move |_| vdom6.weak().render())
        .and_then(move |_| {
            vdom7.weak().with_component(move |comp| {
                let comp = comp.unwrap_mut::<Cached<CountRenders>>();
                assert_eq!(comp.render_count.get(), 2);
                assert_rendered(&container4, &RenderFn(|_| text("2")));
            })
        })
        .map_err(|e| JsValue::from(e.to_string()))
}
