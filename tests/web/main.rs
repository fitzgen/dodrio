//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use bumpalo::Bump;
use dodrio::{Attribute, Node, Render, Vdom};
use futures::prelude::*;
use log::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

pub mod events;
pub mod js_api;
pub mod render;

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

pub fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

pub fn create_element(tag: &str) -> web_sys::Element {
    init_logging();
    document()
        .create_element(tag)
        .expect("should create element OK")
}

/// Ensure that logs go to the devtools console.
pub fn init_logging() {
    use std::sync::Once;
    static START: Once = Once::new();
    START.call_once(|| {
        console_log::init_with_level(Level::Trace).expect("could not initialize console_log");
    });
}

/// Assert that the `container` contains the physical DOM tree that matches
/// `r`'s rendered virtual DOM.
pub fn assert_rendered<R: Render>(container: &web_sys::Element, r: &R) {
    init_logging();

    let bump = &Bump::new();
    let node = r.render(bump);
    let child = container
        .first_child()
        .expect("container does not have anything rendered into it?");
    check_node(&child, &node);

    fn stringify_actual_node(n: &web_sys::Node) -> String {
        if let Some(el) = n.dyn_ref::<web_sys::Element>() {
            el.outer_html()
        } else {
            format!("#text({:?})", n.text_content())
        }
    }

    fn check_node(actual: &web_sys::Node, expected: &Node) {
        debug!("check_render:");
        debug!("    actual = {}", stringify_actual_node(&actual));
        debug!("    expected = {:#?}", expected);
        match expected {
            Node::Text(text_node) => {
                assert_eq!(
                    actual.node_name().to_uppercase(),
                    "#TEXT",
                    "actual.node_name() == #TEXT"
                );
                assert_eq!(
                    actual.text_content().unwrap_or_default(),
                    text_node.text(),
                    "actual.text_content() == expected.text()"
                );
            }
            Node::Element(elem) => {
                assert_eq!(
                    actual.node_name().to_uppercase(),
                    elem.tag_name().to_uppercase(),
                    "actual.node_name() == expected.tag_name()"
                );
                let actual = actual
                    .dyn_ref::<web_sys::Element>()
                    .expect("`actual` should be an `Element`");
                check_attributes(actual.attributes(), elem.attributes());
                check_children(actual.child_nodes(), elem.children());
                if let Some(namespace) = elem.namespace() {
                    assert_eq!(actual.namespace_uri(), Some(namespace.into()))
                }
            }
        }
    }

    fn check_attributes(actual: web_sys::NamedNodeMap, expected: &[Attribute]) {
        assert_eq!(
            actual.length(),
            expected.len() as u32,
            "actual's number of attributes == expected's number of attributes"
        );
        for attr in expected {
            let actual_attr = actual
                .get_named_item(attr.name())
                .expect(&format!("should have attribute \"{}\"", attr.name()));
            assert_eq!(
                actual_attr.value(),
                attr.value(),
                "actual attr value == expected attr value for attr \"{}\"",
                attr.name()
            );
        }
    }

    fn check_children(actual: web_sys::NodeList, expected: &[Node]) {
        assert_eq!(
            actual.length(),
            expected.len() as u32,
            "actual children length == expected children length"
        );
        for (i, child) in expected.iter().enumerate() {
            let actual_child = actual.item(i as u32).unwrap();
            check_node(&actual_child, child);
        }
    }
}

/// Use the function `F` to render.
pub struct RenderFn<F>(F)
where
    F: for<'bump> Fn(&'bump Bump) -> Node<'bump>;

impl<F> Render for RenderFn<F>
where
    F: for<'bump> Fn(&'bump Bump) -> Node<'bump>,
{
    fn render<'bump>(&self, bump: &'bump Bump) -> Node<'bump> {
        (self.0)(bump)
    }
}

/// Assert that if we start by rendering the `before` virtual DOM tree into a
/// physical DOM tree, and then diff it with the `after` virtual DOM tree, then
/// the physical DOM tree correctly matches `after`.
pub fn assert_before_after<R, S>(before: R, after: S) -> impl Future<Item = (), Error = JsValue>
where
    R: 'static + Render,
    S: 'static + Render,
{
    let container = create_element("div");

    let before = Rc::new(before);
    let after = Rc::new(after);

    debug!("====== Rendering the *before* DOM into the physical DOM ======");
    let vdom1 = Rc::new(Vdom::new(&container, before.clone()));
    let vdom2 = vdom1.clone();

    debug!("====== Checking the *before* DOM against the physical DOM ======");
    assert_rendered(&container, &before);

    debug!("====== Rendering the *after* DOM into the physical DOM ======");
    let weak = vdom1.weak();
    weak.set_component(Box::new(after.clone()))
        .map(move |_| {
            debug!("====== Checking the *after* DOM against the physical DOM ======");
            assert_rendered(&container, &after);
            drop(vdom2);
        })
        .map_err(|e| JsValue::from(e.to_string()))
}

/// A helper macro for declaring a bunch of `assert_before_after` tests.
#[macro_export]
macro_rules! before_after {
    ( $(
        $name:ident {
            before($before_bump:ident) {
                $( $before:tt )*
            }
            after($after_bump:ident) {
                $( $after:tt )*
            }
        }
    )* ) => {
        $(
            #[wasm_bindgen_test(async)]
            fn $name() -> impl Future<Item = (), Error = wasm_bindgen::JsValue> {
                use crate::{assert_before_after, RenderFn};
                assert_before_after(
                    RenderFn(|$before_bump| { $( $before )* }),
                    RenderFn(|$after_bump| { $( $after )* })
                )
            }
        )*
    }
}
