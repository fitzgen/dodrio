cfg_if::cfg_if! {
    if #[cfg(feature = "xxx-unstable-strace")] {
        use wasm_bindgen::prelude::*;

        #[wasm_bindgen(module = "/js/strace.js")]
        extern "C" {
            #[wasm_bindgen(js_name = initStrace)]
            fn really_init_strace();
        }

        pub fn init_strace() {
            use std::sync::Once;
            static STRACE: Once = Once::new();
            STRACE.call_once(|| {
                really_init_strace();
            });
        }
    } else {
        #[inline]
        pub fn init_strace() {}
    }
}
