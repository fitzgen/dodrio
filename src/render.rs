use crate::Node;
use bumpalo::Bump;
use std::any::Any;
use std::rc::Rc;
use wasm_bindgen::UnwrapThrowExt;

/// A trait for any component that can be rendered to HTML.
///
/// Takes a shared reference to `self` and generates the virtual DOM that
/// represents its rendered HTML.
///
/// ## `Bump` Allocation
///
/// `Render` implementations can use the provided `Bump` for very fast
/// allocation for anything that needs to be allocated during rendering.
///
/// ## The `'a: 'bump` Lifetime Bound
///
/// The `'a: 'bump` bounds enforce that `self` outlives the given bump
/// allocator. This means that if `self` contains a string, the string does not
/// need to be copied into the output `Node` and can be used by reference
/// instead (i.e. it prevents accidentally using the string after its been
/// freed). The `'a: 'bump` bound also enables abstractions like
/// `dodrio::Cached` that can re-use cached `Node`s across `render`s without
/// copying them.
///
/// ## Example
///
/// ```no_run
/// use dodrio::{bumpalo::Bump, Node, Render};
///
/// pub struct MyComponent;
///
/// impl Render for MyComponent {
///     fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
///     where
///         'a: 'bump
///     {
///         Node::text("This is my component rendered!")
///     }
/// }
/// ```
pub trait Render {
    /// Render `self` as a virtual DOM. Use the given `Bump` for temporary
    /// allocations.
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
