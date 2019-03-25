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
        div(&cx)
            .attr("id", "hello-world")
            .children([text("Hello "), span(&cx).child(text("World!")).finish()])
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
            div(&cx).finish()
        }
    }

    replace_elem_with_text {
        before(cx) {
            div(&cx).finish()
        }
        after(_cx) {
            text("before")
        }
    }

    same_elem {
        before(cx) {
            div(&cx).finish()
        }
        after(cx) {
            div(&cx).finish()
        }
    }

    elems_with_different_tag_names {
        before(cx) {
            span(&cx).finish()
        }
        after(cx) {
            div(&cx).finish()
        }
    }

    same_tag_name_update_attribute {
        before(cx) {
            div(&cx).attr("value", "1").finish()
        }
        after(cx) {
            div(&cx).attr("value", "2").finish()
        }
    }

    same_tag_name_remove_attribute {
        before(cx) {
            div(&cx).attr("value", "1").finish()
        }
        after(cx) {
            div(&cx).finish()
        }
    }

    same_tag_name_add_attribute {
        before(cx) {
            div(&cx).finish()
        }
        after(cx) {
            div(&cx).attr("value", "2").finish()
        }
    }

    same_tag_name_many_attributes {
        before(cx) {
            div(&cx)
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
            div(&cx)
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
            div(&cx).child(text("child")).finish()
        }
        after(cx) {
            div(&cx).child(text("child")).finish()
        }
    }

    same_tag_update_child {
        before(cx) {
            div(&cx).child(text("before")).finish()
        }
        after(cx) {
            div(&cx).child(text("after")).finish()
        }
    }

    same_tag_add_child {
        before(cx) {
            div(&cx).finish()
        }
        after(cx) {
            div(&cx).child(text("child")).finish()
        }
    }

    same_tag_remove_child {
        before(cx) {
            div(&cx).child(text("child")).finish()
        }
        after(cx) {
            div(&cx).finish()
        }
    }

    same_tag_update_many_children {
        before(cx) {
            div(&cx)
                .children([
                    div(&cx).finish(),
                    span(&cx).finish(),
                    p(&cx).finish(),
                ])
                .finish()
        }
        after(cx) {
            div(&cx)
                .children([
                    span(&cx).finish(),
                    p(&cx).finish(),
                    div(&cx).finish(),
                ])
                .finish()
        }
    }

    same_tag_remove_many_children {
        before(cx) {
            div(&cx)
                .children([
                    div(&cx).finish(),
                    span(&cx).finish(),
                    p(&cx).finish(),
                ])
                .finish()
        }
        after(cx) {
            div(&cx)
                .children([
                    div(&cx).finish(),
                ])
                .finish()
        }
    }

    same_tag_add_many_children {
        before(cx) {
            div(&cx)
                .children([
                    div(&cx).finish(),
                ])
                .finish()
        }
        after(cx) {
            div(&cx)
                .children([
                    div(&cx).finish(),
                    span(&cx).finish(),
                    p(&cx).finish(),
                ])
                .finish()
        }
    }

    same_tag_different_namespace {
        before(cx) {
            div(&cx)
                .namespace(Some("http://example.com"))
                .finish()
        }
        after(cx) {
            div(&cx)
                .namespace(Some("http://example.net"))
                .finish()
        }
    }
}
