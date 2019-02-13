use super::{assert_rendered, before_after, create_element, RenderFn};
use dodrio::{builder::*, Vdom};
use futures::prelude::*;
use std::rc::Rc;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn render_initial_text() {
    let hello = Rc::new(RenderFn(|_bump| text("hello")));

    let container = create_element("div");
    let _vdom = Vdom::new(&container, hello.clone());
    assert_rendered(&container, &hello);
}

#[wasm_bindgen_test]
fn render_initial_node() {
    let hello = Rc::new(RenderFn(|bump| {
        div(bump)
            .attr("id", "hello-world")
            .children([text("Hello "), span(bump).child(text("World!")).finish()])
            .finish()
    }));

    let container = create_element("div");
    let _vdom = Vdom::new(&container, hello.clone());
    assert_rendered(&container, &hello);
}

#[wasm_bindgen_test]
fn container_is_emptied_upon_drop() {
    let container = create_element("div");
    let vdom = Vdom::new(&container, RenderFn(|_bump| text("blah")));
    drop(vdom);
    assert!(container.first_child().is_none());
}

before_after! {
    same_text {
        before(_bump) {
            text("hello")
        }
        after(_bump) {
            text("hello")
        }
    }

    update_text {
        before(_bump) {
            text("before")
        }
        after(_bump) {
            text("after")
        }
    }

    replace_text_with_elem {
        before(_bump) {
            text("before")
        }
        after(bump) {
            div(bump).finish()
        }
    }

    replace_elem_with_text {
        before(bump) {
            div(bump).finish()
        }
        after(_bump) {
            text("before")
        }
    }

    same_elem {
        before(bump) {
            div(bump).finish()
        }
        after(bump) {
            div(bump).finish()
        }
    }

    elems_with_different_tag_names {
        before(bump) {
            span(bump).finish()
        }
        after(bump) {
            div(bump).finish()
        }
    }

    same_tag_name_update_attribute {
        before(bump) {
            div(bump).attr("value", "1").finish()
        }
        after(bump) {
            div(bump).attr("value", "2").finish()
        }
    }

    same_tag_name_remove_attribute {
        before(bump) {
            div(bump).attr("value", "1").finish()
        }
        after(bump) {
            div(bump).finish()
        }
    }

    same_tag_name_add_attribute {
        before(bump) {
            div(bump).finish()
        }
        after(bump) {
            div(bump).attr("value", "2").finish()
        }
    }

    same_tag_name_many_attributes {
        before(bump) {
            div(bump)
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
        after(bump) {
            div(bump)
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
        before(bump) {
            div(bump).child(text("child")).finish()
        }
        after(bump) {
            div(bump).child(text("child")).finish()
        }
    }

    same_tag_update_child {
        before(bump) {
            div(bump).child(text("before")).finish()
        }
        after(bump) {
            div(bump).child(text("after")).finish()
        }
    }

    same_tag_add_child {
        before(bump) {
            div(bump).finish()
        }
        after(bump) {
            div(bump).child(text("child")).finish()
        }
    }

    same_tag_remove_child {
        before(bump) {
            div(bump).child(text("child")).finish()
        }
        after(bump) {
            div(bump).finish()
        }
    }

    same_tag_update_many_children {
        before(bump) {
            div(bump)
                .children([
                    div(bump).finish(),
                    span(bump).finish(),
                    p(bump).finish(),
                ])
                .finish()
        }
        after(bump) {
            div(bump)
                .children([
                    span(bump).finish(),
                    p(bump).finish(),
                    div(bump).finish(),
                ])
                .finish()
        }
    }

    same_tag_remove_many_children {
        before(bump) {
            div(bump)
                .children([
                    div(bump).finish(),
                    span(bump).finish(),
                    p(bump).finish(),
                ])
                .finish()
        }
        after(bump) {
            div(bump)
                .children([
                    div(bump).finish(),
                ])
                .finish()
        }
    }

    same_tag_add_many_children {
        before(bump) {
            div(bump)
                .children([
                    div(bump).finish(),
                ])
                .finish()
        }
        after(bump) {
            div(bump)
                .children([
                    div(bump).finish(),
                    span(bump).finish(),
                    p(bump).finish(),
                ])
                .finish()
        }
    }
}
