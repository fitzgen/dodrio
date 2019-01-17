use bumpalo::Bump;
use dodrio::{Attribute, Node, Render};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

struct SayHelloTo {
    who: String,
}

impl SayHelloTo {
    fn new<S: Into<String>>(who: S) -> SayHelloTo {
        let who = who.into();
        SayHelloTo { who }
    }

    fn set_who(&mut self, who: String) {
        self.who = who;
    }
}

impl Render for SayHelloTo {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> dodrio::node::Node<'bump>
    where
        'a: 'bump,
    {
        let input = Node::element(
            bump,
            "input",
            [
                Attribute {
                    name: "type",
                    value: "text",
                },
                Attribute {
                    name: "value",
                    value: &self.who,
                },
            ],
            [],
        );

        let hello = bumpalo::format!(in bump, "Hello, {}!", self.who);
        let hello = Node::text(hello.into_bump_str());

        Node::element(bump, "div", [], [input, hello])
    }
}

#[wasm_bindgen]
pub fn run() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let mut say_hello = SayHelloTo::new("World");
    let mut vdom = dodrio::Vdom::new(&body, &say_hello);

    let on_input = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let input = match event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        {
            Some(input) => input,
            None => return,
        };

        say_hello.set_who(input.value());
        vdom.render(&say_hello);
    }) as Box<FnMut(_)>);

    let _ = body.add_event_listener_with_callback("input", on_input.as_ref().unchecked_ref());
    on_input.forget();
}
