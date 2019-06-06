use crate::{cached_set::CacheId, RootRender, VdomWeak};
use crate::RenderContext;
use crate::Render;
use bumpalo::Bump;
use std::fmt;
use std::iter;
use std::mem;
use std::u32;


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
        Element(&'a ElementNode<'a>),

        /// A node in the vdom's `CachedSet`. This allows us to avoid
        /// re-rendering and re-diffing subtrees.
        Cached(CachedNode),
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
        pub key: NodeKey,
        pub tag_name: &'a str,
        pub listeners: &'a [Listener<'a>],
        pub attributes: &'a [Attribute<'a>],
        pub children: &'a [Node<'a>],
        pub namespace: Option<&'a str>,
    }
}

pub_unstable_internal! {
    /// A cached node exists in an arena that is internal to the `Vdom`. It
    /// allows us to avoid both re-rendering a sub-tree and re-diffing
    /// it. `CachedNode`s cannot contain cycles, since they can only contain
    /// other `CachedNode`s that already exist when they are being constructed.
    #[derive(Debug, Clone, Copy)]
    pub(crate) struct CachedNode {
        pub id: CacheId,
        // Always a copy of the inner cached node's key.
        pub key: NodeKey,
    }
}

/// The key for keyed children.
///
/// Keys must be unique among siblings.
///
/// If any sibling is keyed, then they all must be keyed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey(pub(crate) u32);

impl Default for NodeKey {
    fn default() -> NodeKey {
        NodeKey::NONE
    }
}

impl NodeKey {
    /// The default, lack of a key.
    pub const NONE: NodeKey = NodeKey(u32::MAX);

    /// Is this key `NodeKey::NONE`?
    #[inline]
    pub fn is_none(&self) -> bool {
        *self == Self::NONE
    }

    /// Is this key not `NodeKey::NONE`?
    #[inline]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Create a new `NodeKey`.
    ///
    /// `key` must not be `u32::MAX`.
    #[inline]
    pub fn new(key: u32) -> Self {
        debug_assert_ne!(key, u32::MAX);
        NodeKey(key)
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

impl<'a> From<CachedNode> for Node<'a> {
    #[inline]
    fn from(c: CachedNode) -> Self {
        Node {
            kind: NodeKind::Cached(c),
        }
    }
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
    /// Low-level constructor for making a new `Node` of type element with given
    /// parts.
    ///
    /// This is primarily intended for JSX and templating proc-macros to compile
    /// down into. If you are building nodes by-hand, prefer using the
    /// `dodrio::builder::*` APIs.
    #[inline]
    pub fn element(
        bump: &'a Bump,
        key: NodeKey,
        tag_name: &'a str,
        listeners: &'a [Listener<'a>],
        attributes: &'a [Attribute<'a>],
        children: &'a [Node<'a>],
        namespace: Option<&'a str>,
    ) -> Node<'a> {
        let element = bump.alloc_with(|| ElementNode {
            key,
            tag_name,
            listeners,
            attributes,
            children,
            namespace,
        });

        Node {
            kind: NodeKind::Element(element),
        }
    }

    /// Construct a new text node with the given text.
    #[inline]
    pub(crate) fn text(text: &'a str) -> Node<'a> {
        Node {
            kind: NodeKind::Text(TextNode { text }),
        }
    }

    #[inline]
    pub(crate) fn key(&self) -> NodeKey {
        match &self.kind {
            NodeKind::Text(_) => NodeKey::NONE,
            NodeKind::Element(e) => e.key,
            NodeKind::Cached(c) => c.key,
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

pub fn html_string<R>(component: &R) -> String
where
    R: Render,
{
    let cx = &mut RenderContext::empty();

    let node = component.render(cx);

    let mut s = String::new();
    html_string_recursive(cx, &mut s, &node);
    return s;

    fn html_string_recursive(cx: &mut RenderContext, s: &mut String, node: &Node) {
        match node.kind {
            NodeKind::Text(ref t) => s.push_str(t.text),
            NodeKind::Element(ref e) => {
                s.push_str(e.tag_name);
                for c in e.children {
                    html_string_recursive(cx, s, c);
                }
            }
            NodeKind::Cached(ref c) => {
                let (cache_node, _) = cx.cached_set.borrow().get(c.id);
                html_string_recursive(cx, s, cache_node);
            }
        }
    }
}

