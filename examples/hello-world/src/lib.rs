use dodrio::bumpalo::Bump;
use dodrio::{Node, Render, Vdom};
use wasm_bindgen::prelude::*;

struct Hello<'who> {
    who: &'who str,
}

impl<'who> Hello<'who> {
    fn new(who: &str) -> Hello {
        Hello { who }
    }
}

impl<'who> Render for Hello<'who> {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        Node::element(
            bump,
            "p",
            [],
            [],
            [Node::text("Hello, "), Node::text(self.who), Node::text("!")],
        )
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let body = document.body().unwrap_throw();

    // Create a new `Hello` render component.
    let component = Hello::new("World");

    // Create a virtual DOM and mount it and the `Hello` render component to the
    // `<body>`.
    let vdom = Vdom::new(body.as_ref(), component);

    // Run the virtual DOM forever and don't unmount it.
    vdom.forget();
}
