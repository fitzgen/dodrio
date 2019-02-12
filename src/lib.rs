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

use bumpalo::Bump;
use std::any::Any;
use std::rc::Rc;
use wasm_bindgen::UnwrapThrowExt;

// Only `pub` so that the wasm-bindgen bindings work.
#[doc(hidden)]
pub mod change_list;

mod cached;
mod events;
mod node;
mod vdom;

// Re-export items at the top level.
pub use self::cached::Cached;
pub use self::node::{on, Attribute, ElementNode, Listener, ListenerCallback, Node, TextNode};
pub use self::vdom::{Vdom, VdomWeak};

pub trait Render {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump;
}

impl<'r, R> Render for &'r R
where
    R: Render,
{
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        (**self).render(bump)
    }
}

impl<R> Render for Rc<R>
where
    R: Render,
{
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    where
        'a: 'bump,
    {
        (**self).render(bump)
    }
}

/// A `RootRender` is a render component that can be the root rendering component
/// mounted to a virtual DOM.
///
/// In addition to rendering, it must also be `'static` so that it can be owned
/// by the virtual DOM and `Any` so that it can be downcast to its concrete type
/// by event listener callbacks.
///
/// You do not need to implement this trait by hand: there is a blanket
/// implementation for all `Render` types that fulfill the `RootRender`
/// requirements.
pub trait RootRender: Any + Render {
    /// Get this `&RootRender` trait object as an `&Any` trait object reference.
    fn as_any(&self) -> &Any;

    /// Get this `&mut RootRender` trait object as an `&mut Any` trait object
    /// reference.
    fn as_any_mut(&mut self) -> &mut Any;
}

impl<T: Any + Render> RootRender for T {
    fn as_any(&self) -> &Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut Any {
        self
    }
}

impl dyn RootRender {
    /// Downcast this shared `&dyn RootRender` trait object reference to its
    /// underlying concrete type.
    ///
    /// # Panics
    ///
    /// Panics if this virtual DOM's root rendering component is not an `R`
    /// instance.
    pub fn unwrap_ref<R: RootRender>(&self) -> &R {
        self.as_any()
            .downcast_ref::<R>()
            .expect_throw("bad `RootRender::unwrap_ref` call")
    }

    /// Downcast this exclusive `&mut dyn RootRender` trait object reference to
    /// its underlying concrete type.
    ///
    /// # Panics
    ///
    /// Panics if this virtual DOM's root rendering component is not an `R`
    /// instance.
    pub fn unwrap_mut<R: RootRender>(&mut self) -> &mut R {
        self.as_any_mut()
            .downcast_mut::<R>()
            .expect_throw("bad `RootRender::unwrap_ref` call")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn render_is_object_safe() {
        #[allow(dead_code)]
        fn takes_dyn_render(_: &dyn super::Render) {}
    }

    #[test]
    fn root_render_is_object_safe() {
        #[allow(dead_code)]
        fn takes_dyn_render(_: &dyn super::RootRender) {}
    }
}
