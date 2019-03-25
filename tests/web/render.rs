use super::{assert_rendered, before_after, create_element, RenderFn};
use dodrio::{builder::*, Vdom};
use futures::prelude::*;
use std::rc::Rc;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn render_initial_text() {
    let hello = Rc::new(RenderFn(|_cx| text("hello")));

    let container = create_element("div");
    let _vdom = Vdom::new(&container, hello.clone());
    assert_rendered(&container, &hello);
}

#[wasm_bindgen_test]
fn render_initial_node() {
    let hello = Rc::new(RenderFn(|cx| {
        div(cx.bump)
            .attr("id", "hello-world")
            .children([text("Hello "), span(cx.bump).child(text("World!")).finish()])
            .finish()
    }));

    let container = create_element("div");
    let _vdom = Vdom::new(&container, hello.clone());
    assert_rendered(&container, &hello);
}

#[wasm_bindgen_test]
fn container_is_emptied_upon_drop() {
    let container = create_element("div");
    let vdom = Vdom::new(&container, RenderFn(|_cx| text("blah")));
    drop(vdom);
    assert!(container.first_child().is_none());
}

before_after! {
    same_text {
        before(_cx) {
            text("hello")
        }
        after(_cx) {
            text("hello")
        }
    }

    update_text {
        before(_cx) {
            text("before")
        }
        after(_cx) {
            text("after")
        }
    }

    replace_text_with_elem {
        before(_cx) {
            text("before")
        }
        after(cx) {
            div(cx.bump).finish()
        }
    }

    replace_elem_with_text {
        before(cx) {
            div(cx.bump).finish()
        }
        after(_cx) {
            text("before")
        }
    }

    same_elem {
        before(cx) {
            div(cx.bump).finish()
        }
        after(cx) {
            div(cx.bump).finish()
        }
    }

    elems_with_different_tag_names {
        before(cx) {
            span(cx.bump).finish()
        }
        after(cx) {
            div(cx.bump).finish()
        }
    }

    same_tag_name_update_attribute {
        before(cx) {
            div(cx.bump).attr("value", "1").finish()
        }
        after(cx) {
            div(cx.bump).attr("value", "2").finish()
        }
    }

    same_tag_name_remove_attribute {
        before(cx) {
            div(cx.bump).attr("value", "1").finish()
        }
        after(cx) {
            div(cx.bump).finish()
        }
    }

    same_tag_name_add_attribute {
        before(cx) {
            div(cx.bump).finish()
        }
        after(cx) {
            div(cx.bump).attr("value", "2").finish()
        }
    }

    same_tag_name_many_attributes {
        before(cx) {
            div(cx.bump)
                .attr("before-1", "1")
                .attr("shared-1", "1")
                .attr("modified-1", "1")
                .attr("before-2", "2")
                .attr("shared-2", "2")
                .attr("modified-2", "2")
                .attr("before-3", "3")
                .attr("shared-3", "3")
                .attr("modified-3", "3")
                .finish()
        }
        after(cx) {
            div(cx.bump)
                .attr("after-1", "1")
                .attr("shared-1", "1")
                .attr("modified-1", "100")
                .attr("after-2", "2")
                .attr("shared-2", "2")
                .attr("modified-2", "200")
                .attr("after-3", "3")
                .attr("shared-3", "3")
                .attr("modified-3", "300")
                .finish()
        }
    }

    same_tag_same_children {
        before(cx) {
            div(cx.bump).child(text("child")).finish()
        }
        after(cx) {
            div(cx.bump).child(text("child")).finish()
        }
    }

    same_tag_update_child {
        before(cx) {
            div(cx.bump).child(text("before")).finish()
        }
        after(cx) {
            div(cx.bump).child(text("after")).finish()
        }
    }

    same_tag_add_child {
        before(cx) {
            div(cx.bump).finish()
        }
        after(cx) {
            div(cx.bump).child(text("child")).finish()
        }
    }

    same_tag_remove_child {
        before(cx) {
            div(cx.bump).child(text("child")).finish()
        }
        after(cx) {
            div(cx.bump).finish()
        }
    }

    same_tag_update_many_children {
        before(cx) {
            div(cx.bump)
                .children([
                    div(cx.bump).finish(),
                    span(cx.bump).finish(),
                    p(cx.bump).finish(),
                ])
                .finish()
        }
        after(cx) {
            div(cx.bump)
                .children([
                    span(cx.bump).finish(),
                    p(cx.bump).finish(),
                    div(cx.bump).finish(),
                ])
                .finish()
        }
    }

    same_tag_remove_many_children {
        before(cx) {
            div(cx.bump)
                .children([
                    div(cx.bump).finish(),
                    span(cx.bump).finish(),
                    p(cx.bump).finish(),
                ])
                .finish()
        }
        after(cx) {
            div(cx.bump)
                .children([
                    div(cx.bump).finish(),
                ])
                .finish()
        }
    }

    same_tag_add_many_children {
        before(cx) {
            div(cx.bump)
                .children([
                    div(cx.bump).finish(),
                ])
                .finish()
        }
        after(cx) {
            div(cx.bump)
                .children([
                    div(cx.bump).finish(),
                    span(cx.bump).finish(),
                    p(cx.bump).finish(),
                ])
                .finish()
        }
    }

    same_tag_different_namespace {
        before(cx) {
            div(cx.bump)
                .namespace(Some("http://example.com"))
                .finish()
        }
        after(cx) {
            div(cx.bump)
                .namespace(Some("http://example.net"))
                .finish()
        }
    }
}
