use bumpalo::{Bump, BumpAllocSafe};

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
    pub(crate) attributes: &'a [Attribute<'a>],
    pub(crate) children: &'a [Node<'a>],
}

#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a> Node<'a> {
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

impl<'a> BumpAllocSafe for Node<'a> {}
impl<'a> BumpAllocSafe for Attribute<'a> {}

impl<'a> Node<'a> {
    /// Construct a new text node with the given text.
    #[inline]
    pub fn text(text: &'a str) -> Node<'a> {
        Node::Text(TextNode { text })
    }

    /// Construct a new element node with the given tag name and children.
    #[inline]
    pub fn element<Attributes, Children>(
        bump: &'a Bump,
        tag_name: &'a str,
        attributes: Attributes,
        children: Children,
    ) -> Node<'a>
    where
        Attributes: 'a + BumpAllocSafe + AsRef<[Attribute<'a>]>,
        Children: 'a + BumpAllocSafe + AsRef<[Node<'a>]>,
    {
        let children: &'a Children = bump.alloc(children);
        let children: &'a [Node<'a>] = children.as_ref();

        let attributes: &'a Attributes = bump.alloc(attributes);
        let attributes: &'a [Attribute<'a>] = attributes.as_ref();

        Node::Element(ElementNode {
            tag_name,
            attributes,
            children,
        })
    }
}
