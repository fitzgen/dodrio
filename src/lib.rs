//! The `dodrio` virtual DOM.
//!
//! ## Example
//!
//! ```no_run
//! use dodrio::{on, bumpalo::Bump, Attribute, Node, Render};
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
//! impl<'who> Render for Hello<'who> {
//!     fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
//!     where
//!         'a: 'bump,
//!     {
//!         let id = bumpalo::format!(in bump, "hello-{}", self.who);
//!         Node::element(
//!             bump,
//!             // The element's tag name. In this case a `<p>` element.
//!             "p",
//!             // Listeners.
//!             [on(bump, "click", |root, _vdom, _event| {
//!                 let hello = root.unwrap_mut::<Hello>();
//!                 let window = web_sys::window().expect_throw("should have a `Window` on the Web");
//!                 window.alert_with_message(hello.who);
//!             })],
//!             // Attributes.
//!             [Attribute { name: "id", value: id.into_bump_str() }],
//!             // Child nodes.
//!             [
//!                 Node::text("Hello, "),
//!                 Node::element(
//!                     bump,
//!                     "strong",
//!                     [],
//!                     [],
//!                     [
//!                         Node::text(self.who),
//!                         Node::text("!"),
//!                     ],
//!                 ),
//!             ],
//!         )
//!     }
//! }
//! ```

// Re-export the `bumpalo` crate.
pub use bumpalo;

// Only `pub` so that the wasm-bindgen bindings work.
#[doc(hidden)]
pub mod change_list;

mod cached;
mod events;
mod node;
mod render;
mod vdom;

// Re-export items at the top level.
pub use self::cached::Cached;
pub use self::node::{on, Attribute, ElementNode, Listener, ListenerCallback, Node, TextNode};
pub use self::render::{Render, RootRender};
pub use self::vdom::{Vdom, VdomWeak};
