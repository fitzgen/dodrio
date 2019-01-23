use super::{assert_rendered, before_after, create_element, RenderFn};
use dodrio::{Attribute, Node, Vdom};
use wasm_bindgen_test::*;

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
