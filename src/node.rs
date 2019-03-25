use crate::{RootRender, VdomWeak};
use bumpalo::Bump;
use std::fmt;
use std::iter;
use std::mem;

/// A virtual DOM node.
#[derive(Debug, Clone)]
pub struct Node<'a> {
    #[cfg(feature = "xxx-unstable-internal-use-only")]
    #[doc(hidden)]
    pub kind: NodeKind<'a>,

    #[cfg(not(feature = "xxx-unstable-internal-use-only"))]
    pub(crate) kind: NodeKind<'a>,
}

pub_unstable_internal! {
    /// A node is either a text node or an element.
    #[derive(Debug, Clone)]
    pub(crate) enum NodeKind<'a> {
        /// A text node.
        Text(TextNode<'a>),

        /// An element potentially with attributes and children.
        Element(ElementNode<'a>),
    }
}

pub_unstable_internal! {
    /// Text nodes are just a string of text. They cannot have attributes or
    /// children.
    #[derive(Debug, Clone)]
    pub(crate) struct TextNode<'a> {
        pub text: &'a str,
    }
}

pub_unstable_internal! {
    /// Elements have a tag name, zero or more attributes, and zero or more
    /// children.
    #[derive(Debug, Clone)]
    pub(crate) struct ElementNode<'a> {
        pub tag_name: &'a str,
        pub listeners: &'a [Listener<'a>],
        pub attributes: &'a [Attribute<'a>],
        pub children: &'a [Node<'a>],
        pub namespace: Option<&'a str>,
    }
}

/// An event listener callback function.
///
/// It takes three parameters:
///
/// 1. The virtual DOM's root rendering component.
/// 2. A capability to scheduler virtual DOM re-rendering.
/// 3. The event that occurred.
pub(crate) type ListenerCallback<'a> =
    &'a (dyn Fn(&mut dyn RootRender, VdomWeak, web_sys::Event) + 'static);

/// An event listener.
pub struct Listener<'a> {
    /// The type of event to listen for.
    pub(crate) event: &'a str,
    /// The callback to invoke when the event happens.
    pub(crate) callback: ListenerCallback<'a>,
}

/// An attribute on a DOM node, such as `id="my-thing"` or
/// `href="https://example.com"`.
#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: &'a str,
}

impl fmt::Debug for Listener<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (a, b) = self.get_callback_parts();
        let a = a as *mut u32;
        let b = b as *mut u32;
        f.debug_struct("Listener")
            .field("event", &self.event)
            .field("callback", &(a, b))
            .finish()
    }
}

impl<'a> Attribute<'a> {
    /// Get this attribute's name, such as `"id"` in `<div id="my-thing" />`.
    #[inline]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// The attribute value, such as `"my-thing"` in `<div id="my-thing" />`.
    #[inline]
    pub fn value(&self) -> &'a str {
        self.value
    }

    /// Certain attributes are considered "volatile" and can change via user
    /// input that we can't see when diffing against the old virtual DOM. For
    /// these attributes, we want to always re-set the attribute on the physical
    /// DOM node, even if the old and new virtual DOM nodes have the same value.
    #[inline]
    pub(crate) fn is_volatile(&self) -> bool {
        match self.name {
            "value" | "checked" | "selected" => true,
            _ => false,
        }
    }
}

impl<'a> Node<'a> {
    /// Construct a new Node of type element with given tag name and children
    #[inline]
    pub(crate) fn element<Listeners, Attributes, Children>(
        bump: &'a Bump,
        tag_name: &'a str,
        listeners: Listeners,
        attributes: Attributes,
        children: Children,
        namespace: Option<&'a str>,
    ) -> Node<'a>
    where
        Listeners: 'a + AsRef<[Listener<'a>]>,
        Attributes: 'a + AsRef<[Attribute<'a>]>,
        Children: 'a + AsRef<[Node<'a>]>,
    {
        let children: &'a Children = bump.alloc(children);
        let children: &'a [Node<'a>] = children.as_ref();

        let listeners: &'a Listeners = bump.alloc(listeners);
        let listeners: &'a [Listener<'a>] = listeners.as_ref();

        let attributes: &'a Attributes = bump.alloc(attributes);
        let attributes: &'a [Attribute<'a>] = attributes.as_ref();

        Node {
            kind: NodeKind::Element(ElementNode {
                tag_name,
                listeners,
                attributes,
                children,
                namespace,
            }),
        }
    }

    /// Construct a new text node with the given text.
    #[inline]
    pub(crate) fn text(text: &'a str) -> Node<'a> {
        Node {
            kind: NodeKind::Text(TextNode { text }),
        }
    }
}

/// A node can become an iterator that yields the node itself once.
///
/// This implementation of `IntoIterator` mostly exists to improve the
/// `typed-html` ergonomics, where the macro invokes `.into_iter()` on the child
/// contents of a tag. By implementing `IntoIterator` here, we avoid having to
/// do nasty shenanigans like `<p>vec![$contents]</p>` instead of plain old
/// `<p>$contents</p>`.
impl<'a> IntoIterator for Node<'a> {
    type Item = Node<'a>;
    type IntoIter = iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        iter::once(self)
    }
}

union CallbackFatPtr<'a> {
    callback: ListenerCallback<'a>,
    parts: (u32, u32),
}

impl Listener<'_> {
    #[inline]
    pub(crate) fn get_callback_parts(&self) -> (u32, u32) {
        assert_eq!(
            mem::size_of::<ListenerCallback>(),
            mem::size_of::<CallbackFatPtr>()
        );

        unsafe {
            let fat = CallbackFatPtr {
                callback: self.callback,
            };
            let (a, b) = fat.parts;
            debug_assert!(a != 0);
            (a, b)
        }
    }
}
