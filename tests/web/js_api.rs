use super::{assert_rendered, create_element, RenderFn};
use dodrio::{builder::*, Node, Render, RenderContext, Vdom};
use dodrio_js_api::JsRender;
use futures::channel::oneshot;
use js_sys::Object;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

pub struct WrapJs {
    inner: JsRender,
}

impl WrapJs {
    pub fn new() -> (WrapJs, oneshot::Receiver<()>, Closure<dyn FnMut()>) {
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
        }) as Box<dyn FnMut()>);

        let component = WrapJs {
            inner: JsRender::new(JsComponent::new(&on_after_render)),
        };

        (component, receiver, on_after_render)
    }
}

impl<'a> Render<'a> for WrapJs {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        div(&cx)
            .attr("class", "wrap-js")
            .children([self.inner.render(cx)])
            .finish()
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(extends = Object)]
    #[derive(Clone, Debug)]
    type JsComponent;

    #[wasm_bindgen(constructor)]
    fn new(on_after_render: &Closure<dyn FnMut()>) -> JsComponent;
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

#[wasm_bindgen_test]
async fn can_use_js_rendering_components() {
    eval_js_rendering_component();

    let container = create_element("div");
    let (component, receiver, _closure) = WrapJs::new();
    let _vdom = Rc::new(Vdom::new(&container, component));

    assert_rendered(
        &container,
        &RenderFn(|cx| {
            div(&cx)
                .attr("class", "wrap-js")
                .children([span(&cx)
                    .attr("class", "js-component")
                    .children([
                        text("Here is some plain text"),
                        b(&cx)
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

    receiver.await.unwrap();

    assert_rendered(
        &container,
        &RenderFn(|cx| {
            div(&cx)
                .attr("class", "wrap-js")
                .children([span(&cx)
                    .attr("class", "js-component")
                    .children([
                        text("Here is some plain text"),
                        b(&cx)
                            .children([text("...and here is some bold text")])
                            .finish(),
                        text("1"),
                    ])
                    .finish()])
                .finish()
        }),
    );
}
