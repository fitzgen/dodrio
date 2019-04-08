cfg_if::cfg_if! {
    if #[cfg(all(feature = "xxx-unstable-internal-use-only", not(target_arch = "wasm32")))] {
        #[derive(Clone, Debug)]
        pub struct ChangeListInterpreter {}
        impl ChangeListInterpreter {
            pub fn new(_container: &crate::Element) -> ChangeListInterpreter {
                ChangeListInterpreter {}
            }
            pub fn unmount(&self) {}
            pub fn add_change_list_range(&self, _start: usize, _len: usize) {}
            pub fn init_events_trampoline(&self, _trampoline: &crate::EventsTrampoline) {}
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
        }
    }
}
