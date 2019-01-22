//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use bumpalo::Bump;
use dodrio::{Attribute, Node, Render, Vdom};
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
                .get_named_item(attr.name)
                .expect(&format!("should have attribute \"{}\"", attr.name));
            assert_eq!(
                actual_attr.value(),
                attr.value,
                "actual attr value == expected attr value for attr \"{}\"",
                attr.name
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

struct RenderFn<F>(F)
where
    F: for<'bump> Fn(&'bump Bump) -> Node<'bump>;

impl<F> Render for RenderFn<F>
where
    F: for<'bump> Fn(&'bump Bump) -> Node<'bump>,
{
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        (self.0)(bump)
    }
}

#[wasm_bindgen_test]
fn render_initial_text() {
    let hello = RenderFn(|_bump| Node::text("hello"));

    let container = create_element("div");
    Vdom::new(&container, &hello);
    assert_rendered(&container, &hello);
}

#[wasm_bindgen_test]
fn render_initial_node() {
    let hello = RenderFn(|bump| {
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
    });

    let container = create_element("div");
    Vdom::new(&container, &hello);
    assert_rendered(&container, &hello);
}

fn assert_before_after<R, S>(before: R, after: S)
where
    R: Render,
    S: Render,
{
    let container = create_element("div");

    debug!("====== Rendering the *before* DOM into the physical DOM ======");
    let mut vdom = Vdom::new(&container, &before);
    debug!("====== Checking the *before* DOM against the physical DOM ======");
    assert_rendered(&container, &before);

    debug!("====== Rendering the *after* DOM into the physical DOM ======");
    vdom.render(&after);
    debug!("====== Checking the *after* DOM against the physical DOM ======");
    assert_rendered(&container, &after);
}

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
            fn $name() {
                assert_before_after(
                    RenderFn(|$before_bump| { $( $before )* }),
                    RenderFn(|$after_bump| { $( $after )* })
                );
            }
        )*
    }
}

before_after! {
    same_text {
        before(_bump) {
            Node::text("hello")
        }
        after(_bump) {
            Node::text("hello")
        }
    }

    update_text {
        before(_bump) {
            Node::text("before")
        }
        after(_bump) {
            Node::text("after")
        }
    }

    replace_text_with_elem {
        before(_bump) {
            Node::text("before")
        }
        after(bump) {
            Node::element(bump, "div", [], [])
        }
    }

    replace_elem_with_text {
        before(bump) {
            Node::element(bump, "div", [], [])
        }
        after(_bump) {
            Node::text("before")
        }
    }

    same_elem {
        before(bump) {
            Node::element(bump, "div", [], [])
        }
        after(bump) {
            Node::element(bump, "div", [], [])
        }
    }

    elems_with_different_tag_names {
        before(bump) {
            Node::element(bump, "span", [], [])
        }
        after(bump) {
            Node::element(bump, "div", [], [])
        }
    }

    same_tag_name_update_attribute {
        before(bump) {
            Node::element(bump, "div", [Attribute { name: "value", value: "1" }], [])
        }
        after(bump) {
            Node::element(bump, "div", [Attribute { name: "value", value: "2" }], [])
        }
    }

    same_tag_name_remove_attribute {
        before(bump) {
            Node::element(bump, "div", [Attribute { name: "value", value: "1" }], [])
        }
        after(bump) {
            Node::element(bump, "div", [], [])
        }
    }

    same_tag_name_add_attribute {
        before(bump) {
            Node::element(bump, "div", [], [])
        }
        after(bump) {
            Node::element(bump, "div", [Attribute { name: "value", value: "2" }], [])
        }
    }

    same_tag_name_many_attributes {
        before(bump) {
            Node::element(bump, "div", [
                Attribute { name: "before-1", value: "1" },
                Attribute { name: "shared-1", value: "1" },
                Attribute { name: "modified-1", value: "1" },
                Attribute { name: "before-2", value: "2" },
                Attribute { name: "shared-2", value: "2" },
                Attribute { name: "modified-2", value: "2" },
                Attribute { name: "before-3", value: "3" },
                Attribute { name: "shared-3", value: "3" },
                Attribute { name: "modified-3", value: "3" },
            ], [])
        }
        after(bump) {
            Node::element(bump, "div", [
                Attribute { name: "after-1", value: "1" },
                Attribute { name: "shared-1", value: "1" },
                Attribute { name: "modified-1", value: "100" },
                Attribute { name: "after-2", value: "2" },
                Attribute { name: "shared-2", value: "2" },
                Attribute { name: "modified-2", value: "200" },
                Attribute { name: "after-3", value: "3" },
                Attribute { name: "shared-3", value: "3" },
                Attribute { name: "modified-3", value: "300" },
            ], [])
        }
    }

    same_tag_same_children {
        before(bump) {
            Node::element(bump, "div", [], [
                Node::text("child")
            ])
        }
        after(bump) {
            Node::element(bump, "div", [], [
                Node::text("child")
            ])
        }
    }

    same_tag_update_child {
        before(bump) {
            Node::element(bump, "div", [], [
                Node::text("before")
            ])
        }
        after(bump) {
            Node::element(bump, "div", [], [
                Node::text("after")
            ])
        }
    }

    same_tag_add_child {
        before(bump) {
            Node::element(bump, "div", [], [])
        }
        after(bump) {
            Node::element(bump, "div", [], [
                Node::text("child")
            ])
        }
    }

    same_tag_remove_child {
        before(bump) {
            Node::element(bump, "div", [], [
                Node::text("child")
            ])
        }
        after(bump) {
            Node::element(bump, "div", [], [])
        }
    }

    same_tag_update_many_children {
        before(bump) {
            Node::element(bump, "div", [], [
                Node::element(bump, "div", [], []),
                Node::element(bump, "span", [], []),
                Node::element(bump, "p", [], []),
            ])
        }
        after(bump) {
            Node::element(bump, "div", [], [
                Node::element(bump, "span", [], []),
                Node::element(bump, "p", [], []),
                Node::element(bump, "div", [], []),
            ])
        }
    }

    same_tag_remove_many_children {
        before(bump) {
            Node::element(bump, "div", [], [
                Node::element(bump, "div", [], []),
                Node::element(bump, "span", [], []),
                Node::element(bump, "p", [], []),
            ])
        }
        after(bump) {
            Node::element(bump, "div", [], [
                Node::element(bump, "div", [], []),
            ])
        }
    }

    same_tag_add_many_children {
        before(bump) {
            Node::element(bump, "div", [], [
                Node::element(bump, "div", [], []),
            ])
        }
        after(bump) {
            Node::element(bump, "div", [], [
                Node::element(bump, "div", [], []),
                Node::element(bump, "span", [], []),
                Node::element(bump, "p", [], []),
            ])
        }
    }
}
