use dodrio::{bumpalo, Node, Render, RenderContext};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Say hello to someone.
struct SayHelloTo {
    /// Who to say hello to.
    who: String,
}

impl SayHelloTo {
    /// Construct a new `SayHelloTo` component.
    fn new<S: Into<String>>(who: S) -> SayHelloTo {
        let who = who.into();
        SayHelloTo { who }
    }

    /// Update who to say hello to.
    fn set_who(&mut self, who: String) {
        self.who = who;
    }
}

// The `Render` implementation has a text `<input>` and a `<div>` that shows a
// greeting to the `<input>`'s value.
impl<'a> Render<'a> for SayHelloTo {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;

        div(&cx)
            .children([
                input(&cx)
                    .attr("type", "text")
                    .attr(
                        "value",
                        bumpalo::format!(in cx.bump, "{}", self.who).into_bump_str(),
                    )
                    .on("input", |root, vdom, event| {
                        // If the event's target is our input...
                        let input = match event
                            .target()
                            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        {
                            None => return,
                            Some(input) => input,
                        };

                        // ...then get its value and update who we are greeting.
                        let value = input.value();
                        let hello = root.unwrap_mut::<SayHelloTo>();
                        hello.set_who(value);

                        // Finally, re-render the component on the next animation frame.
                        vdom.schedule_render();
                    })
                    .finish(),
                text(bumpalo::format!(in cx.bump, "Hello, {}!", self.who).into_bump_str()),
            ])
            .finish()
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    // Initialize debugging for when/if something goes wrong.
    console_error_panic_hook::set_once();

    // Get the document's `<body>`.
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let body = document.body().unwrap();

    // Construct a new `SayHelloTo` rendering component.
    let say_hello = SayHelloTo::new("World");

    // Mount the component to the `<body>`.
    let vdom = dodrio::Vdom::new(&body, say_hello);

    // Run the component forever.
    vdom.forget();
}
