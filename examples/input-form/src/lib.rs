use dodrio::bumpalo::{self, Bump};
use dodrio::{on, Attribute, Node, Render};
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
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> dodrio::Node<'bump>
    where
        'a: 'bump,
    {
        let input = Node::element(
            bump,
            "input",
            [on(bump, "input", |root, vdom, event| {
                let input = match event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                {
                    Some(input) => input,
                    None => return,
                };

                root.unwrap_mut::<SayHelloTo>().set_who(input.value());
                vdom.schedule_render();
            })],
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

        Node::element(bump, "div", [], [], [input, hello])
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    let say_hello = SayHelloTo::new("World");

    // Mount the component to the `<body>`.
    let vdom = dodrio::Vdom::new(&body, say_hello);

    // Run the component forever.
    vdom.forget();
}
