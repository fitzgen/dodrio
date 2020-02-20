/*!

Implementing `dodrio` render components with JavaScript.

This crate provides a Rust type `JsRender` that wraps a JavaScript object with a
`render` method. `JsRender` implements `dodrio::Render` by calling its wrapped
object's `render` method to get a JavaScript virtual DOM represented as a tree
of JavaScript values. It then converts this tree of JavaScript values into
`dodrio`'s normal bump-allocated virtual DOM representation.

This is likely much slower than rendering virtual DOMs directly into the bump
allocator from the Rust side of things! Additionally, the shape of the
JavaScript virtual DOM is a bit funky and unidiomatic. Keep in mind that this
crate exists as a proof of concept for integrating JavaScript components into
`dodrio` -- which is itself *also* experimental -- and so this crate definitely
has some rough edges.

# Example

Here is a JavaScript implementation of a rendering component:

```javascript
class Greeting {
  constructor(who) {
    this.who = who;
  }

  render() {
    return {
      tagName: "p",
      attributes: [
        {
          name: "class",
          value: "greeting",
        },
      ],
      listeners: [
        {
          on: "click",
          callback: this.onClick.bind(this),
        }
      ],
      children: [
        "Hello, ",
         {
           tagName: "strong",
           children: [this.who],
         }
      ],
    };
  }

  async onClick(vdom, event) {
    // Be more excited!
    this.who += "!";

    // Schedule a re-render.
    await vdom.render();

    console.log("re-rendering finished!");
  }
}
```

And here is a Rust rendering component that internally uses the JS rendering
component:

```rust,no_run
use dodrio::{Node, Render, RenderContext, Vdom};
use dodrio_js_api::JsRender;
use js_sys::Object;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern {
    // Import the JS `Greeting` class.
    #[wasm_bindgen(extends = Object)]
    #[derive(Clone, Debug)]
    type Greeting;

    // And the `Greeting` class's constructor.
    #[wasm_bindgen(constructor)]
    fn new(who: &str) -> Greeting;
}

/// This is our Rust rendering component that wraps the JS rendering component.
pub struct GreetingViaJs {
    js: JsRender,
}

impl GreetingViaJs {
    /// Create a new `GreetingViaJs`, which will internally create a new JS
    /// `Greeting`.
    pub fn new(who: &str) -> GreetingViaJs {
        let js = JsRender::new(Greeting::new(who));
        GreetingViaJs { js }
    }
}

/// And finally the `Render` implementation! This adds a `<p>` element and some
/// text around whatever the inner JS `Greeting` component renders.
impl<'a> Render<'a> for GreetingViaJs {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;
        p(&cx)
            .children([
                text("JavaScript says: "),
                self.js.render(cx),
            ])
            .finish()
    }
}
```

 */
#![deny(missing_docs, missing_debug_implementations)]

use dodrio::{builder, bumpalo, Node, Render, RenderContext};
use js_sys::{Object, Promise, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    /// A rendering component implemented in JavaScript.
    ///
    /// The rendering API is a bit duck-typed: any JS object with a `render`
    /// method that returns a virtual DOM as JS values with the right shape
    /// works.
    ///
    /// See `JsRender::new` for converting existing JS objects into `JsRender`s.
    #[derive(Debug, Clone)]
    pub type JsRender;
    #[wasm_bindgen(structural, method)]
    fn render(this: &JsRender) -> JsValue;

    #[wasm_bindgen(extends = Object)]
    #[derive(Debug, Clone)]
    type Element;
    #[wasm_bindgen(structural, getter, method, js_name = tagName)]
    fn tag_name(this: &Element) -> String;
    #[wasm_bindgen(structural, getter, method)]
    fn listeners(this: &Element) -> js_sys::Array;
    #[wasm_bindgen(structural, getter, method)]
    fn attributes(this: &Element) -> js_sys::Array;
    #[wasm_bindgen(structural, getter, method)]
    fn children(this: &Element) -> js_sys::Array;

    #[wasm_bindgen(extends = Object)]
    #[derive(Debug, Clone)]
    type Listener;
    #[wasm_bindgen(structural, getter, method)]
    fn on(this: &Listener) -> String;
    #[wasm_bindgen(structural, getter, method)]
    fn callback(this: &Listener) -> js_sys::Function;

    #[wasm_bindgen(extends = Object)]
    #[derive(Debug, Clone)]
    type Attribute;
    #[wasm_bindgen(structural, getter, method)]
    fn name(this: &Attribute) -> String;
    #[wasm_bindgen(structural, getter, method)]
    fn value(this: &Attribute) -> String;
}

/// A weak handle to a virtual DOM.
///
/// This is essentially the same as `dodrio::VdomWeak`, but exposed to
/// JavaScript.
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct VdomWeak {
    inner: dodrio::VdomWeak,
}

impl VdomWeak {
    fn new(inner: dodrio::VdomWeak) -> VdomWeak {
        VdomWeak { inner }
    }
}

#[wasm_bindgen]
impl VdomWeak {
    /// Schedule re-rendering of the virtual DOM. A promise is returned that is
    /// resolved after the rendering has happened.
    pub fn render(&self) -> Promise {
        let future = self.inner.render();

        wasm_bindgen_futures::future_to_promise(async move {
            if let Err(e) = future.await {
                let msg = e.to_string();
                Err(js_sys::Error::new(&msg).into())
            } else {
                Ok(JsValue::null())
            }
        })
    }
}

impl JsRender {
    /// Convert a `js_sys::Object` into a `JsRender`.
    ///
    /// The given object must have a `render` method that conforms to the
    /// duck-typed virtual DOM interface which is described in the crate-level
    /// documentation.
    pub fn new<O>(object: O) -> JsRender
    where
        O: Into<Object>,
    {
        let object = object.into();
        debug_assert!(
            has_property(&object, "render"),
            "JS rendering components must have a `render` method"
        );
        object.unchecked_into::<JsRender>()
    }
}

impl<'a> Render<'a> for JsRender {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        create(cx, self.render())
    }
}

fn has_property(obj: &Object, property: &str) -> bool {
    Reflect::has(obj, &property.into()).unwrap_or_default()
}

fn create<'a>(cx: &mut RenderContext<'a>, val: JsValue) -> Node<'a> {
    if let Some(txt) = val.as_string() {
        let text = bumpalo::collections::String::from_str_in(&txt, cx.bump);
        return builder::text(text.into_bump_str());
    }

    let elem = val.unchecked_into::<Element>();
    debug_assert!(
        elem.is_instance_of::<Object>(),
        "JS render methods should only return strings for text nodes or objects for elements"
    );
    debug_assert!(
        has_property(&elem, "tagName"),
        "element objects returned by JS render methods must have a `tagName` property"
    );

    let tag_name = elem.tag_name();
    let tag_name = bumpalo::collections::String::from_str_in(&tag_name, cx.bump);

    builder::ElementBuilder::new(cx.bump, tag_name.into_bump_str())
        .listeners({
            let mut listeners =
                bumpalo::collections::Vec::new_in(cx.bump);
            if has_property(&elem, "listeners") {
                let js_listeners = elem.listeners();
                listeners.reserve(js_listeners.length() as usize);
                js_listeners.for_each(&mut |listener, _index, _array| {
                    let listener = listener.unchecked_into::<Listener>();
                    debug_assert!(
                        listener.is_instance_of::<Object>(),
                        "listeners returned by JS render methods must be objects"
                    );
                    debug_assert!(
                        has_property(&listener, "on"),
                        "listener objects returned by JS render methods must have an `on` property"
                    );
                    debug_assert!(
                        has_property(&listener, "callback"),
                        "listener objects returned by JS render methods must have an `callback` property"
                    );
                    let on = listener.on();
                    let on = bumpalo::collections::String::from_str_in(&on, cx.bump);
                    let callback = listener.callback();
                    let elem = elem.clone();
                    listeners.push(builder::on(cx.bump, on.into_bump_str(), move |_root, vdom, event| {
                        let vdom = VdomWeak::new(vdom);
                        let vdom: JsValue = vdom.into();
                        if let Err(e) = callback.call2(&elem, &vdom, &event) {
                            wasm_bindgen::throw_val(e);
                        }
                    }));
                });
            }
            listeners
        })
        .attributes({
            let mut attributes = bumpalo::collections::Vec::new_in(cx.bump);
            if has_property(&elem, "attributes") {
                let js_attributes = elem.attributes();
                attributes.reserve(js_attributes.length() as usize);
                js_attributes.for_each(&mut |attribute, _index, _array| {
                    let attribute = attribute.unchecked_into::<Attribute>();
                    debug_assert!(
                        attribute.is_instance_of::<Object>(),
                        "attributes returned by JS render methods must be objects"
                    );
                    debug_assert!(
                        has_property(&attribute, "name"),
                        "attribute objects returned by JS render methods must have a `name` property"
                    );
                    debug_assert!(
                        has_property(&attribute, "value"),
                        "attribute objects returned by JS render methods must have a `value` property"
                    );
                    let name = attribute.name();
                    let name = bumpalo::collections::String::from_str_in(&name, cx.bump);
                    let value = attribute.value();
                    let value = bumpalo::collections::String::from_str_in(&value, cx.bump);
                    attributes.push(builder::attr(name.into_bump_str(), value.into_bump_str()));
                });
            }
            attributes
        })
        .children({
            let mut children = bumpalo::collections::Vec::new_in(cx.bump);
            if has_property(&elem, "children") {
                let js_children = elem.children();
                children.reserve(js_children.length() as usize);
                js_children.for_each(&mut |child, _index, _array| {
                    children.push(create(cx, child));
                });
            }
            children
        })
        .finish()
}
