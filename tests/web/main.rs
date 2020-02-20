//! Test suite for the Web and headless browsers.

#![cfg(all(feature = "xxx-unstable-internal-use-only", target_arch = "wasm32"))]

use bumpalo::Bump;
use dodrio::{
    Attribute, CachedSet, ElementNode, Node, NodeKind, Render, RenderContext, TextNode, Vdom,
};
use fxhash::FxHashMap;
use log::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

pub mod cached;
pub mod events;
pub mod js_api;
pub mod keyed;
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
pub fn assert_rendered<R: for<'a> Render<'a>>(container: &web_sys::Element, r: &R) {
    init_logging();

    let cached_set = &RefCell::new(CachedSet::default());
    let bump = &Bump::new();
    let templates = &mut FxHashMap::default();
    let cx = &mut RenderContext::new(bump, cached_set, templates);
    let node = r.render(cx);
    let child = container
        .first_child()
        .expect("container does not have anything rendered into it?");

    let cached_set = cached_set.borrow();
    check_node(&cached_set, &child, &node);

    fn stringify_actual_node(n: &web_sys::Node) -> String {
        if let Some(el) = n.dyn_ref::<web_sys::Element>() {
            el.outer_html()
        } else {
            format!("#text({:?})", n.text_content())
        }
    }

    fn check_node(cached_set: &CachedSet, actual: &web_sys::Node, expected: &Node) {
        debug!("check_render:");
        debug!("    actual = {}", stringify_actual_node(&actual));
        debug!("    expected = {:#?}", expected);
        match expected.kind {
            NodeKind::Text(TextNode { text }) => {
                assert_eq!(
                    actual.node_name().to_uppercase(),
                    "#TEXT",
                    "actual.node_name() == #TEXT"
                );
                assert_eq!(
                    actual.text_content().unwrap_or_default(),
                    text,
                    "actual.text_content() == expected.text()"
                );
            }
            NodeKind::Element(&ElementNode {
                tag_name,
                attributes,
                children,
                namespace,
                ..
            }) => {
                assert_eq!(
                    actual.node_name().to_uppercase(),
                    tag_name.to_uppercase(),
                    "actual.node_name() == expected.tag_name()"
                );
                let actual = actual
                    .dyn_ref::<web_sys::Element>()
                    .expect("`actual` should be an `Element`");
                check_attributes(actual.attributes(), attributes);
                check_children(cached_set, actual.child_nodes(), children);
                if let Some(namespace) = namespace {
                    assert_eq!(actual.namespace_uri(), Some(namespace.into()))
                }
            }
            NodeKind::Cached(ref c) => {
                let (expected, _template) = cached_set.get(c.id);
                check_node(cached_set, actual, &expected);
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

    fn check_children(cached_set: &CachedSet, actual: web_sys::NodeList, expected: &[Node]) {
        assert_eq!(
            actual.length(),
            expected.len() as u32,
            "actual children length == expected children length"
        );
        for (i, child) in expected.iter().enumerate() {
            let actual_child = actual.item(i as u32).unwrap();
            check_node(cached_set, &actual_child, child);
        }
    }
}

/// Use the function `F` to render.
pub struct RenderFn<F>(F)
where
    F: for<'a> Fn(&mut RenderContext<'a>) -> Node<'a>;

impl<'a, F> Render<'a> for RenderFn<F>
where
    F: for<'b> Fn(&mut RenderContext<'b>) -> Node<'b>,
{
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        (self.0)(cx)
    }
}

/// Assert that if we start by rendering the `before` virtual DOM tree into a
/// physical DOM tree, and then diff it with the `after` virtual DOM tree, then
/// the physical DOM tree correctly matches `after`.
pub async fn assert_before_after<R, S>(before: R, after: S) -> Result<(), JsValue>
where
    R: 'static + for<'a> Render<'a>,
    S: 'static + for<'a> Render<'a>,
{
    let container = create_element("div");

    let before = Rc::new(before);
    let after = Rc::new(after);

    debug!("====== Rendering the *before* DOM into the physical DOM ======");
    let vdom1 = Rc::new(Vdom::new(&container, before.clone()));
    let _vdom2 = vdom1.clone();

    debug!("====== Checking the *before* DOM against the physical DOM ======");
    assert_rendered(&container, &before);

    debug!("====== Rendering the *after* DOM into the physical DOM ======");
    let weak = vdom1.weak();
    weak.set_component(Box::new(after.clone()))
        .await
        .map_err(|e| e.to_string())?;

    debug!("====== Checking the *after* DOM against the physical DOM ======");
    assert_rendered(&container, &after);

    Ok(())
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
            #[wasm_bindgen_test]
            async fn $name() {
                use crate::{assert_before_after, RenderFn};
                log::debug!("############### {} ###############", stringify!($name));
                assert_before_after(
                    RenderFn(|$before_bump| { $( $before )* }),
                    RenderFn(|$after_bump| { $( $after )* })
                )
                .await
                .unwrap()
            }
        )*
    }
}
