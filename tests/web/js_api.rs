use super::{assert_rendered, create_element, RenderFn};
use dodrio::{builder::*, bumpalo::Bump, Node, Render, Vdom};
use dodrio_js_api::JsRender;
use futures::prelude::*;
use futures::sync::oneshot;
use js_sys::Object;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

pub struct WrapJs {
    inner: JsRender,
}

impl WrapJs {
    pub fn new() -> (WrapJs, oneshot::Receiver<()>, Closure<FnMut()>) {
        let (sender, receiver) = oneshot::channel();

        let on_after_render = Closure::wrap(Box::new({
            let mut sender = Some(sender);
            move || {
                sender
                    .take()
                    .expect_throw("on_after_render should only be called once")
                    .send(())
                    .expect_throw("receiver should not have been dropped");
            }
        }) as Box<FnMut()>);

        let component = WrapJs {
            inner: JsRender::new(JsComponent::new(&on_after_render)),
        };

        (component, receiver, on_after_render)
    }
}

impl Render for WrapJs {
    fn render<'bump>(&self, bump: &'bump Bump) -> Node<'bump> {
        div(bump)
            .attr("class", "wrap-js")
            .children([self.inner.render(bump)])
            .finish()
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = Object)]
    #[derive(Clone, Debug)]
    type JsComponent;

    #[wasm_bindgen(constructor)]
    fn new(on_after_render: &Closure<FnMut()>) -> JsComponent;
}

fn eval_js_rendering_component() {
    use std::sync::Once;
    static EVAL: Once = Once::new();
    EVAL.call_once(|| {
        super::init_logging();
        js_sys::eval(
            r#"
            window.JsComponent = class JsComponent {
                constructor(onAfterRender) {
                    this.count = 0;
                    this.onAfterRender = onAfterRender;
                }

                render() {
                    return {
                        tagName: "span",
                        attributes: [{ name: "class", value: "js-component" }],
                        listeners: [{ on: "click", callback: this.onClick.bind(this) }],
                        children: [
                            "Here is some plain text",
                            {
                                tagName: "b",
                                children: ["...and here is some bold text"]
                            },
                            String(this.count),
                        ]
                    };
                }

                async onClick(vdom, event) {
                    this.count++;
                    await vdom.render();
                    this.onAfterRender();
                }
            };
            "#,
        )
        .expect_throw("should eval JS component OK");
    });
}

#[wasm_bindgen_test(async)]
fn can_use_js_rendering_components() -> impl Future<Item = (), Error = JsValue> {
    eval_js_rendering_component();

    let container = create_element("div");
    let (component, receiver, closure) = WrapJs::new();

    let vdom = Rc::new(Vdom::new(&container, component));
    assert_rendered(
        &container,
        &RenderFn(|bump| {
            div(bump)
                .attr("class", "wrap-js")
                .children([span(bump)
                    .attr("class", "js-component")
                    .children([
                        text("Here is some plain text"),
                        b(bump)
                            .children([text("...and here is some bold text")])
                            .finish(),
                        text("0"),
                    ])
                    .finish()])
                .finish()
        }),
    );

    container
        .query_selector(".js-component")
        .expect_throw("should querySelector OK")
        .expect_throw("should find `.js-component` element OK")
        .dyn_into::<web_sys::HtmlElement>()
        .expect_throw("should be an `HTMLElement` object")
        .click();

    receiver
        .map(move |_| {
            assert_rendered(
                &container,
                &RenderFn(|bump| {
                    div(bump)
                        .attr("class", "wrap-js")
                        .children([span(bump)
                            .attr("class", "js-component")
                            .children([
                                text("Here is some plain text"),
                                b(bump)
                                    .children([text("...and here is some bold text")])
                                    .finish(),
                                text("1"),
                            ])
                            .finish()])
                        .finish()
                }),
            );

            drop(vdom);
            drop(container);
            drop(closure);
        })
        .map_err(|e| e.to_string().into())
}
