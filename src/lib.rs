use bumpalo::Bump;

// Only `pub` so that the wasm-bindgen bindings work.
#[doc(hidden)]
pub mod change_list;

mod cached;
mod node;
mod vdom;

// Re-export items at the top level.
pub use self::cached::Cached;
pub use self::node::{Attribute, ElementNode, Node, TextNode};
pub use self::vdom::Vdom;

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
