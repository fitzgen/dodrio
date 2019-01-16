//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use bumpalo::Bump;
use dodrio::{
    node::{Attribute, Node},
    vdom::Vdom,
    Render,
};
use log::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

fn create_element(tag: &str) -> web_sys::Element {
    document()
        .create_element(tag)
        .expect("should create element OK")
}

fn init_logging() {
    use std::sync::Once;
    static START: Once = Once::new();
    START.call_once(|| {
        console_log::init_with_level(Level::Trace).expect("could not initialize console_log");
    });
}

fn assert_rendered<R: Render>(container: &web_sys::Element, r: &R) {
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
        info!("check_render:");
        info!("    actual = {}", stringify_actual_node(&actual));
        info!("    expected = {:?}", expected);
        match expected {
            Node::Text(text_node) => {
                assert_eq!(actual.node_name().to_uppercase(), "#TEXT");
                assert_eq!(actual.text_content().unwrap_or_default(), text_node.text());
            }
            Node::Element(elem) => {
                assert_eq!(
                    actual.node_name().to_uppercase(),
                    elem.tag_name().to_uppercase()
                );
                let actual = actual
                    .dyn_ref::<web_sys::Element>()
                    .expect("`actual` should be an `Element`");
                check_attributes(actual.attributes(), elem.attributes());
                check_children(actual.child_nodes(), elem.children());
            }
        }
    }

    fn check_attributes(actual: web_sys::NamedNodeMap, expected: &[Attribute]) {
        assert_eq!(actual.length(), expected.len() as u32);
        for attr in expected {
            let actual_attr = actual
                .get_named_item(attr.name)
                .expect(&format!("should have attribute \"{}\"", attr.name));
            assert_eq!(actual_attr.value(), attr.value);
        }
    }

    fn check_children(actual: web_sys::NodeList, expected: &[Node]) {
        assert_eq!(actual.length(), expected.len() as u32);
        for (i, child) in expected.iter().enumerate() {
            let actual_child = actual.item(i as u32).unwrap();
            check_node(&actual_child, child);
        }
    }
}

#[wasm_bindgen_test]
fn render_initial_text() {
    struct Hello;

    impl Render for Hello {
        fn render<'a, 'bump>(&'a self, _bump: &'bump Bump) -> Node<'bump>
        where
            'a: 'bump,
        {
            Node::text("Hello")
        }
    }

    let container = create_element("div");
    Vdom::new(&container, &Hello);
    assert_rendered(&container, &Hello);
}

#[wasm_bindgen_test]
fn render_initial_node() {
    struct Hello;

    impl Render for Hello {
        fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
        where
            'a: 'bump,
        {
            Node::element(
                bump,
                "div",
                [Attribute {
                    name: "id",
                    value: "hello-world",
                }],
                [
                    Node::text("Hello "),
                    Node::element(bump, "span", [], [Node::text("World!")]),
                ],
            )
        }
    }

    let container = create_element("div");
    Vdom::new(&container, &Hello);
    assert_rendered(&container, &Hello);
}
