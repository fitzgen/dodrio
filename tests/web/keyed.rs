use crate::{assert_rendered, create_element};
use dodrio::{builder::*, bumpalo, Node, Render, RenderContext, Vdom};
use log::*;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

struct Keyed(u16);

impl<'a> Render<'a> for Keyed {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        let key = bumpalo::format!(in cx.bump, "{}", self.0).into_bump_str();
        div(&cx)
            .attr("class", "keyed")
            .attr("id", key)
            .key(self.0 as u32)
            .finish()
    }
}

fn keyed<'a, Keys>(cx: &mut RenderContext<'a>, keys: Keys) -> Node<'a>
where
    Keys: AsRef<[u16]>,
{
    let mut parent = div(&cx).attr("class", "parent");

    for &k in keys.as_ref() {
        parent = parent.child(Keyed(k).render(cx));
    }

    parent.finish()
}

async fn assert_keyed<Before, After>(before: Before, after: After) -> Result<(), JsValue>
where
    Before: 'static + for<'a> Render<'a>,
    After: 'static + for<'a> Render<'a>,
{
    #[wasm_bindgen(module = "/tests/web/keyed.js")]
    extern "C" {
        #[wasm_bindgen(js_name = saveKeyedElements)]
        fn save_keyed_elements(container: &web_sys::Element) -> JsValue;

        #[wasm_bindgen(js_name = checkKeyedElements)]
        fn check_keyed_elements(container: &web_sys::Element, saved: JsValue);
    }

    let container = create_element("div");

    let before = Rc::new(before);
    let after = Rc::new(after);

    debug!("====== Rendering the *before* DOM into the physical DOM ======");
    let vdom1 = Rc::new(Vdom::new(&container, before.clone()));
    let _vdom2 = vdom1.clone();
    let saved = save_keyed_elements(&container);

    debug!("====== Checking the *before* DOM against the physical DOM ======");
    assert_rendered(&container, &before);

    debug!("====== Rendering the *after* DOM into the physical DOM ======");
    let weak = vdom1.weak();
    weak.set_component(Box::new(after.clone()))
        .await
        .map_err(|e| e.to_string())?;

    debug!("====== Checking the *after* DOM against the physical DOM ======");
    assert_rendered(&container, &after);
    check_keyed_elements(&container, saved);

    Ok(())
}

macro_rules! keyed_tests {
    ( $(
        $name:ident {
            before($before_cx:ident) {
                $( $before:tt )*
            }
            after($after_cx:ident) {
                $( $after:tt )*
            }
        }
    )* ) => {
        $(
            #[wasm_bindgen_test]
            async fn $name() {
                use crate::RenderFn;
                log::debug!("############### {} ###############", stringify!($name));
                assert_keyed(
                    RenderFn(|$before_cx| { $( $before )* }),
                    RenderFn(|$after_cx| { $( $after )* }),
                )
                .await
                .unwrap()
            }
        )*
    }
}

keyed_tests! {
    same_order {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [1, 2, 3])
        }
    }

    same_order_append {
        before(cx) {
            keyed(cx, [1])
        }
        after(cx) {
            keyed(cx, [1, 2])
        }
    }

    same_order_delete {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [1, 2])
        }
    }

    same_suffix {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [4, 2, 3])
        }
    }

    same_prefix_and_suffix_reorder_middle {
        before(cx) {
            keyed(cx, [1, 2, 3, 4])
        }
        after(cx) {
            keyed(cx, [1, 3, 2, 4])
        }
    }

    same_prefix_and_suffix_new_middle {
        before(cx) {
            keyed(cx, [2, 3, 4, 5])
        }
        after(cx) {
            keyed(cx, [2, 7, 8, 5])
        }
    }

    reverse_order {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [3, 2, 1])
        }
    }

    no_shared_keys {
        before(cx) {
            keyed(cx, [1, 2])
        }
        after(cx) {
            keyed(cx, [3, 4])
        }
    }

    new_keys_in_middle {
        before(cx) {
            keyed(cx, [1, 2])
        }
        after(cx) {
            keyed(cx, [1, 3, 4, 2])
        }
    }

    new_keys_at_start_and_end {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [4, 1, 2, 3, 5])
        }
    }

    delete_prefix {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [2, 3])
        }
    }

    delete_suffix {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [1, 2])
        }
    }

    delete_middle {
        before(cx) {
            keyed(cx, [1, 2, 3])
        }
        after(cx) {
            keyed(cx, [1, 3])
        }
    }

    nested_keyed_children {
        before(cx) {
            ul(&cx)
                .children([
                    li(&cx)
                        .key(1)
                        .children([
                            keyed(cx, [2, 3, 4])
                        ])
                        .finish(),
                    li(&cx)
                        .key(5)
                        .children([
                            keyed(cx, [6, 7, 8])
                        ])
                        .finish(),
                    li(&cx)
                        .key(9)
                        .children([
                            keyed(cx, [10, 11, 12])
                        ])
                        .finish(),
                ])
                .finish()
        }
        after(cx) {
            ul(&cx)
                .children([
                    li(&cx)
                        .key(9)
                        .children([
                            keyed(cx, [12, 11, 10])
                        ])
                        .finish(),
                    li(&cx)
                        .key(5)
                        .children([
                            keyed(cx, [8, 7, 6])
                        ])
                        .finish(),
                    li(&cx)
                        .key(1)
                        .children([
                            keyed(cx, [4, 3, 2])
                        ])
                        .finish(),
                ])
                .finish()
        }
    }
}
