use bumpalo::Bump;

#[macro_use]
mod macros;

pub mod change_list;
pub mod node;
pub mod vdom;

pub use self::node::{Attribute, Node, NodeRef};
pub use self::vdom::Vdom;

pub trait Render {
    fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> NodeRef<'bump>
    where
        'a: 'bump;
}
