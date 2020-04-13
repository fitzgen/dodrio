cfg_if::cfg_if! {
    if #[cfg(all(feature = "xxx-unstable-internal-use-only", not(target_arch = "wasm32")))] {
        use wasm_bindgen::prelude::JsValue;

        #[derive(Clone, Debug)]
        pub struct ChangeListInterpreter {}
        impl ChangeListInterpreter {
            pub fn new(_container: &crate::Element) -> ChangeListInterpreter {
                ChangeListInterpreter {}
            }
            pub fn unmount(&self) {}
            pub fn add_change_list_range(&self, _start: usize, _len: usize) {}
            pub fn init_events_trampoline(&self, _trampoline: &crate::EventsTrampoline) {}
            pub fn start(&self) {}
            pub fn reset(&self) {}

            // -- ops

            // 0
            pub fn set_text(&self, pointer: u32, len: u32, memory: JsValue) {}
            // 1
            pub fn remove_self_and_next_siblings(&self) {}
            // 2
            pub fn replace_with(&self) {}
            // 3
            pub fn set_attribute(&self, name_id: u32, value_id: u32) {}
            // 4
            pub fn remove_attribute(&self, name_id: u32) {}
            // 5
            pub fn push_reverse_child(&self, n: u32) {}
            // 6
            pub fn pop_push_child(&self, n: u32) {}
            // 7
            pub fn pop(&self) {}
            // 8
            pub fn append_child(&self) {}
            // 9
            pub fn create_text_node(&self, pointer: u32, len: u32, memory: JsValue) {}
            // 10
            pub fn create_element(&self, tag_name_id: u32) {}
            // 11
            pub fn new_event_listener(&self, event_id: u32, a: u32, b: u32) {}
            // 12
            pub fn update_event_listener(&self, event_id: u32, a: u32, b: u32) {}
            // 13
            pub fn remove_event_listener(&self, event_id: u32) {}
            // 14
            pub fn add_cached_string(&self, pointer: u32, len: u32, id: u32, memory: JsValue) {}
            // 15
            pub fn drop_cached_string(&self, id: u32) {}
            // 16
            pub fn create_element_ns(&self, tag_name_id: u32, ns_id: u32) {}
            // 17
            pub fn save_children_to_temporaries(&self, temp: u32, start: u32, end: u32) {}
            // 18
            pub fn push_child(&self, n: u32) {}
            // 19
            pub fn push_temporary(&self, temp: u32) {}
            // 20
            pub fn insert_before(&self) {}
            // 21
            pub fn pop_push_reverse_child(&self, n: u32) {}
            // 22
            pub fn remove_child(&self, n: u32) {}
            // 23
            pub fn set_class(&self, class_id: u32) {}
            // 24
            pub fn save_template(&self, id: u32) {}
            // 25
            pub fn push_template(&self, id: u32) {}
        }
    } else {
        use wasm_bindgen::prelude::*;

        #[wasm_bindgen(module = "/js/change-list-interpreter.js")]
        extern "C" {
            #[derive(Clone, Debug)]
            pub type ChangeListInterpreter;

            #[wasm_bindgen(constructor)]
            pub fn new(container: &web_sys::Element) -> ChangeListInterpreter;

            #[wasm_bindgen(structural, method)]
            pub fn unmount(this: &ChangeListInterpreter);

            #[wasm_bindgen(structural, method, js_name = addChangeListRange)]
            pub fn add_change_list_range(this: &ChangeListInterpreter, start: usize, len: usize);

            #[wasm_bindgen(structural, method, js_name = applyChanges)]
            pub fn apply_changes(this: &ChangeListInterpreter, memory: JsValue);

            #[wasm_bindgen(structural, method, js_name = initEventsTrampoline)]
            pub fn init_events_trampoline(
                this: &ChangeListInterpreter,
                trampoline: &crate::EventsTrampoline,
            );

            #[wasm_bindgen(structural, method, js_name = start)]
            pub fn start(this: &ChangeListInterpreter);

            #[wasm_bindgen(structural, method, js_name = reset)]
            pub fn reset(this: &ChangeListInterpreter);

            // -- ops

            // 0
            #[wasm_bindgen(structural, method, js_name = setText)]
            pub fn set_text(this: &ChangeListInterpreter, pointer: u32, len: u32, memory: JsValue);
            // 1
            #[wasm_bindgen(structural, method, js_name = removeSelfAndNextSiblings)]
            pub fn remove_self_and_next_siblings(this: &ChangeListInterpreter);
            // 2
            #[wasm_bindgen(structural, method, js_name = replaceWith)]
            pub fn replace_with(this: &ChangeListInterpreter);
            // 3
            #[wasm_bindgen(structural, method, js_name = setAttribute)]
            pub fn set_attribute(this: &ChangeListInterpreter, name_id: u32, value_id: u32);
            // 4
            #[wasm_bindgen(structural, method, js_name = removeAttribute)]
            pub fn remove_attribute(this: &ChangeListInterpreter, name_id: u32);
            // 5
            #[wasm_bindgen(structural, method, js_name = pushReverseChild)]
            pub fn push_reverse_child(this: &ChangeListInterpreter, n: u32);
            // 6
            #[wasm_bindgen(structural, method, js_name = popPushChild)]
            pub fn pop_push_child(this: &ChangeListInterpreter, n: u32);
            // 7
            #[wasm_bindgen(structural, method, js_name = pop)]
            pub fn pop(this: &ChangeListInterpreter);
            // 8
            #[wasm_bindgen(structural, method, js_name = appendChild)]
            pub fn append_child(this: &ChangeListInterpreter);
            // 9
            #[wasm_bindgen(structural, method, js_name = createTextNode)]
            pub fn create_text_node(this: &ChangeListInterpreter, pointer: u32, len: u32, memory: JsValue);
            // 10
            #[wasm_bindgen(structural, method, js_name = createElement)]
            pub fn create_element(this: &ChangeListInterpreter, tag_name_id: u32);
            // 11
            #[wasm_bindgen(structural, method, js_name = newEventListener)]
            pub fn new_event_listener(this: &ChangeListInterpreter, event_id: u32, a: u32, b: u32);
            // 12
            #[wasm_bindgen(structural, method, js_name = updateEventListener)]
            pub fn update_event_listener(this: &ChangeListInterpreter, event_id: u32, a: u32, b: u32);
            // 13
            #[wasm_bindgen(structural, method, js_name = removeEventListener)]
            pub fn remove_event_listener(this: &ChangeListInterpreter, event_id: u32);
            // 14
            #[wasm_bindgen(structural, method, js_name = addCachedString)]
            pub fn add_cached_string(this: &ChangeListInterpreter, pointer: u32, len: u32, id: u32, memory: JsValue);
            // 15
            #[wasm_bindgen(structural, method, js_name = dropCachedString)]
            pub fn drop_cached_string(this: &ChangeListInterpreter, id: u32);
            // 16
            #[wasm_bindgen(structural, method, js_name = createElementNS)]
            pub fn create_element_ns(this: &ChangeListInterpreter, tag_name_id: u32, ns_id: u32);
            // 17
            #[wasm_bindgen(structural, method, js_name = saveChildrenToTemporaries)]
            pub fn save_children_to_temporaries(this: &ChangeListInterpreter, temp: u32, start: u32, end: u32);
            // 18
            #[wasm_bindgen(structural, method, js_name = pushChild)]
            pub fn push_child(this: &ChangeListInterpreter, n: u32);
            // 19
            #[wasm_bindgen(structural, method, js_name = pushTemporary)]
            pub fn push_temporary(this: &ChangeListInterpreter, temp: u32);
            // 20
            #[wasm_bindgen(structural, method, js_name = insertBefore)]
            pub fn insert_before(this: &ChangeListInterpreter);
            // 21
            #[wasm_bindgen(structural, method, js_name = popPushReverseChild)]
            pub fn pop_push_reverse_child(this: &ChangeListInterpreter, n: u32);
            // 22
            #[wasm_bindgen(structural, method, js_name = removeChild)]
            pub fn remove_child(this: &ChangeListInterpreter, n: u32);
            // 23
            #[wasm_bindgen(structural, method, js_name = setClass)]
            pub fn set_class(this: &ChangeListInterpreter, class_id: u32);
            // 24
            #[wasm_bindgen(structural, method, js_name = saveTemplate)]
            pub fn save_template(this: &ChangeListInterpreter, id: u32);
            // 25
            #[wasm_bindgen(structural, method, js_name = pushTemplate)]
            pub fn push_template(this: &ChangeListInterpreter, id: u32);
        }
    }
}
