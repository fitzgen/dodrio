use bumpalo::Bump;
use dodrio::{Attribute, Node, Render};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

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
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> dodrio::node::Node<'bump>
    where
        'a: 'bump,
    {
        let val = bumpalo::format!(in bump, "{}", self.val);

        let increment = Node::element(
            bump,
            "button",
            [Attribute {
                name: "data-action",
                value: "increment",
            }],
            [Node::text("+")],
        );

        let decrement = Node::element(
            bump,
            "button",
            [Attribute {
                name: "data-action",
                value: "decrement",
            }],
            [Node::text("-")],
        );

        Node::element(
            bump,
            "div",
            [],
            [increment, Node::text(val.into_bump_str()), decrement],
        )
    }
}

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let mut counter = Counter::new();
    let mut vdom = dodrio::Vdom::new(&body, &counter);

    let on_click = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        match event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
            .and_then(|e| e.get_attribute("data-action"))
        {
            Some(ref s) if s == "increment" => counter.increment(),
            Some(ref s) if s == "decrement" => counter.decrement(),
            _ => {}
        }
        vdom.render(&counter);
    }) as Box<FnMut(_)>);

    let _ = body.add_event_listener_with_callback("click", on_click.as_ref().unchecked_ref());
    on_click.forget();
}
