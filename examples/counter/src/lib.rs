use dodrio::bumpalo::{self, Bump};
use dodrio::{on, Node, Render};
use log::*;
use wasm_bindgen::prelude::*;

struct Counter {
    val: isize,
}

impl Counter {
    fn new() -> Counter {
        Counter { val: 0 }
    }

    fn increment(&mut self) {
        self.val += 1;
    }

    fn decrement(&mut self) {
        self.val -= 1;
    }
}

impl Render for Counter {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> dodrio::Node<'bump>
    where
        'a: 'bump,
    {
        let val = bumpalo::format!(in bump, "{}", self.val);

        Node::element(
            bump,
            "div",
            [],
            [],
            [
                Node::element(
                    bump,
                    "button",
                    [on(bump, "click", |root, vdom, _event| {
                        root.unwrap_mut::<Counter>().increment();
                        vdom.schedule_render();
                    })],
                    [],
                    [Node::text("+")],
                ),
                Node::text(val.into_bump_str()),
                Node::element(
                    bump,
                    "button",
                    [on(bump, "click", |root, vdom, _event| {
                        root.unwrap_mut::<Counter>().decrement();
                        vdom.schedule_render();
                    })],
                    [],
                    [Node::text("-")],
                ),
            ],
        )
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Trace).expect("should initialize logging OK");

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    // Construct a new counter component.
    let counter = Counter::new();

    // Mount our counter component to the `<body>`.
    let vdom = dodrio::Vdom::new(&body, counter);

    // Run the virtual DOM and its listeners forever.
    vdom.forget();
}
