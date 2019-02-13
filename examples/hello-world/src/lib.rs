use dodrio::bumpalo::Bump;
use dodrio::{Node, Render, Vdom};
use wasm_bindgen::prelude::*;

/// A rendering component that displays a greeting.
struct Hello<'who> {
    /// Who to greet.
    who: &'who str,
}

// The `Render` implementation describes how to render a `Hello` component into
// HTML.
impl<'who> Render for Hello<'who> {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        use dodrio::builder::*;
        p(bump)
            .children([text("Hello, "), text(self.who), text("!")])
            .finish()
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    // Set up the panic hook for debugging when things go wrong.
    console_error_panic_hook::set_once();

    // Grab the document's `<body>`.
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let body = document.body().unwrap_throw();

    // Create a new `Hello` render component.
    let component = Hello { who: "World" };

    // Create a virtual DOM and mount it and the `Hello` render component to the
    // `<body>`.
    let vdom = Vdom::new(body.as_ref(), component);

    // Run the virtual DOM forever and don't unmount it.
    vdom.forget();
}
