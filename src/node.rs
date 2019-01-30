use crate::{RootRender, VdomWeak};
use bumpalo::{Bump, BumpAllocSafe};
use std::fmt;
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
    pub event: &'a str,
    /// The callback to invoke when the event happens.
    pub callback: ListenerCallback<'a>,
}

#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a> BumpAllocSafe for Node<'a> {}
impl<'a> BumpAllocSafe for Listener<'a> {}
impl<'a> BumpAllocSafe for Attribute<'a> {}

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

impl<'a> Node<'a> {
    /// Construct a new element node with the given tag name and children.
    #[inline]
    pub fn element<Listeners, Attributes, Children>(
        bump: &'a Bump,
        tag_name: &'a str,
        listeners: Listeners,
        attributes: Attributes,
        children: Children,
    ) -> Node<'a>
    where
        Listeners: 'a + BumpAllocSafe + AsRef<[Listener<'a>]>,
        Attributes: 'a + BumpAllocSafe + AsRef<[Attribute<'a>]>,
        Children: 'a + BumpAllocSafe + AsRef<[Node<'a>]>,
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
    pub fn text(text: &'a str) -> Node<'a> {
        Node::Text(TextNode { text })
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

/// Utility function for creating event listeners and downcasting the component
/// to its `RootRender` concrete type.
pub fn on<'a, F>(bump: &'a Bump, event: &'a str, callback: F) -> Listener<'a>
where
    F: Fn(&mut dyn RootRender, VdomWeak, web_sys::Event) + 'static,
{
    Listener {
        event,
        callback: bump.alloc(
            move |component: &mut dyn RootRender, vdom: VdomWeak, event: web_sys::Event| {
                callback(component, vdom, event);
            },
        ),
    }
}
