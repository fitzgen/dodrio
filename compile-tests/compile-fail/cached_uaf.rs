extern crate bumpalo;
extern crate dodrio;
extern crate web_sys;

use bumpalo::Bump;
use dodrio::{Cached, Node, Render, Vdom};
use std::cell::Cell;

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

fn use_after_free_with_cached_components() {
    let container = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .create_element("div")
        .unwrap();

    // Create a component that caches its own rendered results.
    let mut cached = Cached::new(CountRenders::new());

    // The vdom's current root is pointing into cached's bump memory.
    let mut vdom = Vdom::new(&container, &cached);

    // Drop the cached component and its bump memory.
    drop(cached);
    //~^ ERROR

    // Rendering a new component will diff against the current component's last
    // render, which will be stored in the (now freed) bump inside
    // `Cached`. Yikes!
    vdom.render_component(&CountRenders::new());
}

fn main() {}
