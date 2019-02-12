//! TodoMVC implemented in `dodrio`!

#![deny(missing_docs)]

pub mod controller;
pub mod keys;
pub mod router;
pub mod todo;
pub mod todos;
pub mod utils;
pub mod visibility;

use controller::Controller;
use dodrio::Vdom;
use todos::Todos;
use wasm_bindgen::prelude::*;

/// Run the TodoMVC app!
///
/// Since this is marked `#[wasm_bindgen(start)]` it is automatically invoked
/// once the wasm module instantiated on the Web page.
#[wasm_bindgen(start)]
pub fn run() {
    // Set up the logging for debugging if/when things go wrong.
    init_logging();

    // Grab the TODO app container.
    let document = utils::document();
    let container = document
        .query_selector(".todoapp")
        .unwrap_throw()
        .unwrap_throw();

    // Create a new `Todos` render component.
    let todos = Todos::<Controller>::new();

    // Create a virtual DOM and mount it and the `Todos` render component.
    let vdom = Vdom::new(&container, todos);

    // Start the URL router.
    router::start(vdom.weak());

    // Run the virtual DOM forever and don't unmount it.
    vdom.forget();
}

cfg_if::cfg_if! {
    if #[cfg(feature = "logging")] {
        fn init_logging() {
            use log::*;
            console_error_panic_hook::set_once();
            console_log::init_with_level(Level::Trace).expect_throw("should initialize logging OK");
        }
    } else {
        fn init_logging() {
            // Do nothing.
        }
    }
}
