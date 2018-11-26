use bumpalo::BumpAllocSafe;

#[derive(Clone, Debug)]
pub struct Node<'a, Attributes, Children> {
    data: NodeData<'a>,
    attributes: Attributes,
    children: Children,
}

#[derive(Clone, Copy, Debug)]
pub enum NodeData<'a> {
    Element { tag_name: &'a str },
    Text { text: &'a str },
}

#[derive(Clone, Debug)]
pub struct NodeRef<'a> {
    pub(crate) data: NodeData<'a>,
    pub(crate) attributes: &'a [Attribute<'a>],
    pub(crate) children: &'a [NodeRef<'a>],
}

#[derive(Clone, Debug)]
pub struct Attribute<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

impl<'a, Attributes, Children> BumpAllocSafe for Node<'a, Attributes, Children>
where
    Attributes: BumpAllocSafe,
    Children: BumpAllocSafe,
{
}

impl<'a, Attributes, Children> From<&'a Node<'a, Attributes, Children>> for NodeRef<'a>
where
    Attributes: AsRef<[Attribute<'a>]>,
    Children: AsRef<[NodeRef<'a>]>,
{
    fn from(node: &'a Node<'a, Attributes, Children>) -> NodeRef<'a> {
        NodeRef {
            data: node.data,
            attributes: node.attributes.as_ref(),
            children: node.children.as_ref(),
        }
    }
}

impl<'a, Attributes, Children> From<&'a mut Node<'a, Attributes, Children>> for NodeRef<'a>
where
    Attributes: AsRef<[Attribute<'a>]>,
    Children: AsRef<[NodeRef<'a>]>,
{
    fn from(node: &'a mut Node<'a, Attributes, Children>) -> NodeRef<'a> {
        NodeRef {
            data: node.data,
            attributes: node.attributes.as_ref(),
            children: node.children.as_ref(),
        }
    }
}

impl<'a, Attributes, Children> Node<'a, Attributes, Children>
where
    Attributes: AsRef<[Attribute<'a>]>,
    Children: AsRef<[NodeRef<'a>]>,
{
    pub fn element(
        tag_name: &'a str,
        attributes: Attributes,
        children: Children,
    ) -> Node<'a, Attributes, Children> {
        let data = NodeData::Element { tag_name };
        Node {
            data,
            attributes,
            children,
        }
    }
}

impl<'a> Node<'a, [Attribute<'a>; 0], [NodeRef<'a>; 0]> {
    pub fn text(text: &'a str) -> Node<'a, [Attribute<'a>; 0], [NodeRef<'a>; 0]> {
        let data = NodeData::Text { text };
        let attributes = [];
        let children = [];
        Node {
            data,
            attributes,
            children,
        }
    }
}
