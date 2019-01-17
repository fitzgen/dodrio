use bumpalo::Bump;
use dodrio::{Node, Render};
use wasm_bindgen::prelude::*;

struct HelloWorld<'a>(&'a str);

impl<'who> HelloWorld<'who> {
    fn new(who: &str) -> HelloWorld {
        HelloWorld(who)
    }
}

impl<'who> Render for HelloWorld<'who> {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> dodrio::node::Node<'bump>
    where
        'a: 'bump,
    {
        Node::element(
            bump,
            "p",
            [],
            [Node::text("Hello, "), Node::text(self.0), Node::text("!")],
        )
    }
}

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    // Create a new dodrio vdom contained in the body, with an initial virtual
    // dom.
    let mut vdom = dodrio::Vdom::new(body.as_ref(), &HelloWorld::new("World"));

    // Render a new node tree into the virtual dom.
    vdom.render(&HelloWorld::new("Dodrio"));
}
