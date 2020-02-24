//! The `dodrio` virtual DOM.
//!
//! ## Example
//!
//! ```no_run
//! use dodrio::{bumpalo, Attribute, Node, Render, RenderContext};
//! use wasm_bindgen::UnwrapThrowExt;
//!
//! /// A component that greets someone.
//! pub struct Hello<'who> {
//!     who: &'who str,
//! }
//!
//! impl<'who> Hello<'who> {
//!     /// Construct a new `Hello` component that greets the given `who`.
//!     pub fn new(who: &str) -> Hello {
//!         Hello { who }
//!     }
//! }
//!
//! impl<'a, 'who> Render<'a> for Hello<'who> {
//!     fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
//!         use dodrio::builder::*;
//!
//!         let id = bumpalo::format!(in cx.bump, "hello-{}", self.who);
//!         let who = bumpalo::collections::String::from_str_in(self.who, cx.bump).into_bump_str();
//!
//!         div(&cx)
//!            .attr("id", id.into_bump_str())
//!            .on("click", |root, _vdom, _event| {
//!                 let hello = root.unwrap_mut::<Hello>();
//!                 web_sys::window()
//!                     .expect_throw("should have a `Window` on the Web")
//!                     .alert_with_message(hello.who);
//!             })
//!             .children([
//!                 text("Hello, "),
//!                 strong(&cx)
//!                     .children([
//!                         text(who),
//!                         text("!"),
//!                     ])
//!                     .finish(),
//!             ])
//!             .finish()
//!     }
//! }
//! ```
#![deny(missing_docs, missing_debug_implementations)]

// Re-export the `bumpalo` crate.
pub use bumpalo;

cfg_if::cfg_if! {
    if #[cfg(feature = "log")] {
        #[macro_use]
        extern crate log;
    } else {
        #[macro_use]
        mod logging;
    }
}

// A macro to expose items unstably, only for internal/testing usage.
cfg_if::cfg_if! {
    if #[cfg(feature = "xxx-unstable-internal-use-only")] {
        macro_rules! pub_unstable_internal {
            ( $(#[$attr:meta])* pub(crate) $( $thing:tt )* ) => {
                #[doc(hidden)]
                $( #[$attr] )*
                pub $( $thing )*
            }
        }
    } else {
        macro_rules! pub_unstable_internal {
            ( $(#[$attr:meta])* pub(crate) $( $thing:tt )* ) => {
                $( #[$attr] )*
                pub(crate) $( $thing )*
            }
        }
    }
}

// Only `pub` so that the wasm-bindgen bindings work.
#[doc(hidden)]
pub mod change_list;

mod cached;
mod cached_set;
mod diff;
mod events;
mod node;
mod render;
mod render_context;
mod strace;
mod vdom;

pub mod builder;

// Re-export items at the top level.
pub use self::cached::Cached;
pub use self::node::{Attribute, Listener, Node, NodeKey};
pub use self::render::{Render, RootRender};
pub use self::render_context::RenderContext;
pub use self::vdom::{Vdom, VdomWeak};

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", not(feature = "xxx-unstable-internal-use-only")))] {
        use wasm_bindgen::__rt::WasmRefCell as RefCell;
    } else {
        use std::cell::RefCell;
    }
}

// Polyfill some Web stuff for benchmarking...
cfg_if::cfg_if! {
    if #[cfg(all(feature = "xxx-unstable-internal-use-only", not(target_arch = "wasm32")))] {
        /// An element node in the physical DOM.
        pub type Element = ();

        pub(crate) type EventsTrampoline = ();
    } else {
        /// An element node in the physical DOM.
        pub type Element = web_sys::Element;

        pub(crate) type EventsTrampoline = wasm_bindgen::closure::Closure<dyn Fn(web_sys::Event, u32, u32)>;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "xxx-unstable-internal-use-only")] {
        pub use self::cached_set::{CachedSet};
        pub use self::node::{ElementNode, NodeKind, TextNode};
    }
}
