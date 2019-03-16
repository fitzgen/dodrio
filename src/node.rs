use crate::{RootRender, VdomWeak};
use bumpalo::Bump;
use std::fmt;
use std::iter;
use std::mem;

/// A node is either a text node or an element.
#[derive(Debug, Clone)]
pub enum Node<'a> {
    /// A text node.
    Text(TextNode<'a>),

    /// An element potentially with attributes and children.
    Element(ElementNode<'a>),
}

/// Text nodes are just a string of text. They cannot have attributes or
/// children.
#[derive(Debug, Clone)]
pub struct TextNode<'a> {
    pub(crate) text: &'a str,
}

/// Elements have a tag name, zero or more attributes, and zero or more
/// children.
#[derive(Debug, Clone)]
pub struct ElementNode<'a> {
    pub(crate) tag_name: &'a str,
    pub(crate) listeners: &'a [Listener<'a>],
    pub(crate) attributes: &'a [Attribute<'a>],
    pub(crate) children: &'a [Node<'a>],
}

/// An event listener callback function.
///
/// It takes three parameters:
///
/// 1. The virtual DOM's root rendering component.
/// 2. A capability to scheduler virtual DOM re-rendering.
/// 3. The event that ocurred.
pub type ListenerCallback<'a> =
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
    /// Construct a new element node with the given tag name and children.
    #[inline]
    pub(crate) fn element<Listeners, Attributes, Children>(
        bump: &'a Bump,
        tag_name: &'a str,
        listeners: Listeners,
        attributes: Attributes,
        children: Children,
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

        Node::Element(ElementNode {
            tag_name,
            listeners,
            attributes,
            children,
        })
    }

    /// Is this node a text node?
    pub fn is_text(&self) -> bool {
        match self {
            Node::Text(_) => true,
            _ => false,
        }
    }

    /// Is this node an element?
    pub fn is_element(&self) -> bool {
        match self {
            Node::Element { .. } => true,
            _ => false,
        }
    }

    /// Construct a new text node with the given text.
    #[inline]
    pub(crate) fn text(text: &'a str) -> Node<'a> {
        Node::Text(TextNode { text })
    }
}

impl<'a> IntoIterator for Node<'a> {
    type Item = Node<'a>;
    type IntoIter = iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        iter::once(self)
    }
}

impl<'a> TextNode<'a> {
    /// Get this text node's text content.
    pub fn text(&self) -> &'a str {
        self.text
    }
}

impl<'a> ElementNode<'a> {
    /// Get this element's tag name.
    pub fn tag_name(&self) -> &'a str {
        self.tag_name
    }

    /// Get this element's attributes.
    pub fn attributes(&self) -> &'a [Attribute<'a>] {
        self.attributes
    }

    /// Get this element's attributes.
    pub fn children(&self) -> &'a [Node<'a>] {
        self.children
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
