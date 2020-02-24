use super::{assert_rendered, before_after, create_element, RenderFn};
use dodrio::{builder::*, bumpalo, Cached, Node, Render, RenderContext, RootRender, Vdom};
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

#[derive(Default)]
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

impl<'a> Render<'a> for CountRenders {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        let count = self.render_count.get() + 1;
        self.render_count.set(count);

        let s = bumpalo::format!(in cx.bump, "{}", count);
        text(s.into_bump_str())
    }
}

#[wasm_bindgen_test]
async fn uses_cached_render() {
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

    // We render, populate the cache, and get "1".
    vdom0.weak().render().await.unwrap();
    vdom0
        .weak()
        .with_component(move |comp| {
            let comp = comp.unwrap_mut::<Cached<CountRenders>>();
            assert_eq!(comp.render_count.get(), 1);
            assert_rendered(&container1, &RenderFn(|_| text("1")));
        })
        .await
        .unwrap();

    // We re-render, re-use the cached node, and get "1" again.
    vdom1.weak().render().await.unwrap();
    vdom2
        .weak()
        .with_component(move |comp| {
            let comp = comp.unwrap_mut::<Cached<CountRenders>>();
            assert_eq!(comp.render_count.get(), 1);
            assert_rendered(&container2, &RenderFn(|_| text("1")));
        })
        .await
        .unwrap();

    // We invalidate the cache, re-render, re-populate the cache, and should now have "2".
    vdom3
        .weak()
        .with_component(move |comp| {
            let comp = comp.unwrap_mut::<Cached<CountRenders>>();
            Cached::invalidate(comp);
        })
        .await
        .unwrap();
    vdom4.weak().render().await.unwrap();
    vdom5
        .weak()
        .with_component(move |comp| {
            let comp = comp.unwrap_mut::<Cached<CountRenders>>();
            assert_eq!(comp.render_count.get(), 2);
            assert_rendered(&container3, &RenderFn(|_| text("2")));
        })
        .await
        .unwrap();

    // We re-render, re-use the cached node, and get "2" again.
    vdom6.weak().render().await.unwrap();
    vdom7
        .weak()
        .with_component(move |comp| {
            let comp = comp.unwrap_mut::<Cached<CountRenders>>();
            assert_eq!(comp.render_count.get(), 2);
            assert_rendered(&container4, &RenderFn(|_| text("2")));
        })
        .await
        .unwrap();
}

#[wasm_bindgen(module = "/tests/web/cached.js")]
extern "C" {
    #[wasm_bindgen(js_name = getCloneNodeCount)]
    fn get_clone_node_count() -> u32;
}

#[wasm_bindgen_test]
async fn creating_cached_nodes_uses_clone_node() {
    use dodrio::builder::*;

    let container = create_element("div");

    // The initial vdom render should populate the cache and save a template.
    let vdom0 = Rc::new(Vdom::new(&container, Cached::new(CountRenders::new())));
    let vdom1 = vdom0.clone();
    let _vdom2 = vdom0.clone();

    let clone_node_count_before = get_clone_node_count();

    // Render something entirely different...
    vdom0
        .weak()
        .set_component(Box::new(RenderFn(|_| text("hi"))) as Box<dyn RootRender>)
        .await
        .unwrap();

    // Re-render our cached node. Since we are creating it from scratch,
    // and already have a template from the earlier render, we should
    // call `cloneNode` here.
    vdom1
        .weak()
        .set_component(Box::new(Cached::new(CountRenders::new())) as Box<dyn RootRender>)
        .await
        .unwrap();

    let clone_node_count_after = get_clone_node_count();
    assert_eq!(clone_node_count_after, clone_node_count_before + 1);
}

struct Id(&'static str);

impl Default for Id {
    fn default() -> Id {
        Id("")
    }
}

impl<'a> Render<'a> for Id {
    fn render(&self, _cx: &mut RenderContext<'a>) -> Node<'a> {
        text(self.0)
    }
}

thread_local! {
    static WARM_CHEESE: Rc<Cached<Id>> = Rc::new(Cached::new(Id("cheese")));
    static WARM_CHEESIER: Rc<Cached<Id>> = Rc::new(Cached::new(Id("cheesier")));
}

fn warm_cheese<'a>(cx: &mut RenderContext<'a>) -> Node<'a> {
    WARM_CHEESE.with(|c| {
        let _ = c.render(cx);
        c.render(cx)
    })
}

fn warm_cheesier<'a>(cx: &mut RenderContext<'a>) -> Node<'a> {
    WARM_CHEESIER.with(|c| {
        let _ = c.render(cx);
        c.render(cx)
    })
}

before_after! {
    cold_cache_and_not_cached {
        before(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
        after(cx) {
            Id("ravioli").render(cx)
        }
    }

    not_cached_and_cold_cache {
        before(cx) {
            Id("ravioli").render(cx)
        }
        after(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
    }

    cold_cache_and_cold_cache_same {
        before(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
        after(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
    }

    cold_cache_and_cold_cache_different {
        before(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
        after(cx) {
            Cached::new(Id("ravioli")).render(cx)
        }
    }

    warm_cache_and_not_cached {
        before(cx) {
            warm_cheese(cx)
        }
        after(cx) {
            Id("ravioli").render(cx)
        }
    }

    not_cached_and_warm_cache {
        before(cx) {
            Id("ravioli").render(cx)
        }
        after(cx) {
            warm_cheese(cx)
        }
    }

    warm_cache_and_warm_cache_same {
        before(cx) {
            warm_cheese(cx)
        }
        after(cx) {
            warm_cheese(cx)
        }
    }

    warm_cache_and_warm_cache_different {
        before(cx) {
            warm_cheese(cx)
        }
        after(cx) {
            warm_cheesier(cx)
        }
    }

    cold_cache_and_warm_cache_same {
        before(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
        after(cx) {
            warm_cheese(cx)
        }
    }

    cold_cache_and_warm_cache_different {
        before(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
        after(cx) {
            warm_cheesier(cx)
        }
    }

    warm_cache_and_cold_cache_same {
        before(cx) {
            warm_cheese(cx)
        }
        after(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
    }

    warm_cache_and_cold_cache_different {
        before(cx) {
            warm_cheesier(cx)
        }
        after(cx) {
            Cached::new(Id("cheese")).render(cx)
        }
    }
}
