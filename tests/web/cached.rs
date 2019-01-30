use super::{assert_rendered, create_element, RenderFn};
use bumpalo::Bump;
use dodrio::{Cached, Node, Render, Vdom};
use std::cell::Cell;
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
        Node::text(s.into_bump_str())
    }
}

#[wasm_bindgen_test]
fn uses_cached_render() {
    let container = create_element("div");
    let cached = Cached::new(CountRenders::new());
    let mut vdom = Vdom::new(&container, cached);

    for _ in 0..10 {
        vdom.render();
        assert_eq!(vdom.component().render_count.get(), 1);
        assert_rendered(&container, &RenderFn(|_| Node::text("1")))
    }

    Cached::invalidate(vdom.component_mut());

    for _ in 0..10 {
        vdom.render();
        assert_eq!(vdom.component().render_count.get(), 2);
        assert_rendered(&container, &RenderFn(|_| Node::text("2")))
    }
}
