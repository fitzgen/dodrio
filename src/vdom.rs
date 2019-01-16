use super::change_list::ChangeList;
use super::node::{Attribute, ElementNode, Node, TextNode};
use super::Render;
use bumpalo::Bump;
use log::*;
use std::cmp;
use std::mem;

pub struct Vdom {
    dom_buffers: [Bump; 2],
    change_list: ChangeList,
    container: web_sys::Element,

    // Actually a reference into `self.dom_buffers[0]`.
    current_root: Option<Node<'static>>,
}

unsafe fn extend_node_lifetime<'a>(node: Node<'a>) -> Node<'static> {
    mem::transmute(node)
}

impl Vdom {
    pub fn new<R>(container: &web_sys::Element, contents: &R) -> Vdom
    where
        R: Render,
    {
        let dom_buffers = [Bump::new(), Bump::new()];
        let change_list = ChangeList::new(container);

        // Ensure that the container is empty.
        container.set_inner_html("");

        // Create a dummy `<div/>` in our container.
        let current_root = Node::element(&dom_buffers[0], "div", [], []);
        let current_root = Some(unsafe { extend_node_lifetime(current_root) });
        let window = web_sys::window().expect("should have acess to the Window");
        let document = window
            .document()
            .expect("should have access to the Document");
        container
            .append_child(
                document
                    .create_element("div")
                    .expect("should create element OK")
                    .as_ref(),
            )
            .expect("should append child OK");

        // Diff and apply the `contents` against our dummy `<div/>`.
        let container = container.clone();
        let mut vdom = Vdom {
            dom_buffers,
            change_list,
            container,
            current_root,
        };
        vdom.render(contents);
        vdom
    }

    pub fn container(&self) -> &web_sys::Element {
        &self.container
    }

    pub fn render<R>(&mut self, contents: &R)
    where
        R: Render,
    {
        unsafe {
            self.dom_buffers[1].reset();
            let new_contents = contents.render(&self.dom_buffers[1]);
            let new_contents = extend_node_lifetime(new_contents);

            let old_contents = self.current_root.take().unwrap();
            self.diff(old_contents, new_contents.clone());

            self.swap_buffers();
            self.set_current_root(new_contents);

            self.change_list.apply_changes();
        }
    }

    fn swap_buffers(&mut self) {
        let (first, second) = self.dom_buffers.as_mut().split_at_mut(1);
        mem::swap(&mut first[0], &mut second[0]);
    }

    unsafe fn set_current_root(&mut self, current: Node<'static>) {
        debug_assert!(self.current_root.is_none());
        self.current_root = Some(current);
    }

    fn diff(&self, old: Node, new: Node) {
        // debug!("---------------------------------------------------------");
        // debug!("dodrio::Vdom::diff");
        // debug!("  old = {:#?}", old);
        // debug!("  new = {:#?}", new);
        match (&new, old) {
            (&Node::Text(TextNode { text: new_text }), Node::Text(TextNode { text: old_text })) => {
                debug!("  both are text nodes");
                if new_text != old_text {
                    debug!("  text needs updating");
                    self.change_list.emit_set_text(new_text);
                }
            }
            (&Node::Text(TextNode { .. }), Node::Element(ElementNode { .. })) => {
                debug!("  replacing a text node with an element");
                self.create(new);
                self.change_list.emit_replace_with();
            }
            (&Node::Element(ElementNode { .. }), Node::Text(TextNode { .. })) => {
                debug!("  replacing an element with a text node");
                self.create(new);
                self.change_list.emit_replace_with();
            }
            (
                &Node::Element(ElementNode {
                    tag_name: new_tag_name,
                    attributes: new_attributes,
                    children: new_children,
                }),
                Node::Element(ElementNode {
                    tag_name: old_tag_name,
                    attributes: old_attributes,
                    children: old_children,
                }),
            ) => {
                debug!("  updating an element");
                if new_tag_name != old_tag_name {
                    debug!("  different tag names; creating new element and replacing old element");
                    self.create(new);
                    self.change_list.emit_replace_with();
                    return;
                }
                self.diff_attributes(old_attributes, new_attributes);
                self.diff_children(old_children, new_children);
            }
        }
    }

    fn diff_attributes(&self, old: &[Attribute], new: &[Attribute]) {
        debug!("  updating attributes");

        // Do O(n^2) passes to add/update and remove attributes, since
        // there are almost always very few attributes.
        'outer: for new_attr in new {
            for old_attr in old {
                if old_attr.name == new_attr.name {
                    if old_attr.value != new_attr.value {
                        self.change_list
                            .emit_set_attribute(new_attr.name, new_attr.value);
                    }
                    continue 'outer;
                }
            }
            self.change_list
                .emit_set_attribute(new_attr.name, new_attr.value);
        }

        'outer2: for old_attr in old {
            for new_attr in new {
                if old_attr.name == new_attr.name {
                    continue 'outer2;
                }
            }
            self.change_list.emit_remove_attribute(old_attr.name);
        }
    }

    fn diff_children(&self, old: &[Node], new: &[Node]) {
        debug!("  updating children shared by old and new");

        let num_children_to_diff = cmp::min(new.len(), old.len());
        let mut new_children = new.iter();
        let mut old_children = old.iter();
        let mut pushed = false;

        for (i, (new_child, old_child)) in new_children
            .by_ref()
            .zip(old_children.by_ref())
            .take(num_children_to_diff)
            .enumerate()
        {
            if i == 0 {
                self.change_list.emit_push_first_child();
                pushed = true;
            } else {
                self.change_list.emit_pop_push_next_sibling();
            }

            self.diff(old_child.clone(), new_child.clone());
        }

        debug!("  removing extra old children");

        if old_children.next().is_some() {
            if !pushed {
                self.change_list.emit_push_first_child();
                pushed = true;
            }
            self.change_list.emit_remove_self_and_next_siblings();
        }

        debug!("  creating new children");

        for (i, new_child) in new_children.enumerate() {
            if i == 0 && pushed {
                self.change_list.emit_pop();
                pushed = false;
            }
            self.create(new_child.clone());
            self.change_list.emit_append_child();
        }

        // TODO FITZGEN: only if we pushed?
        debug!("  done updating children");
        if pushed {
            self.change_list.emit_pop();
        }
    }

    fn create(&self, node: Node) {
        // debug!("dodrio::Vdom::create({:#?})", node);
        match node {
            Node::Text(TextNode { text }) => {
                self.change_list.emit_create_text_node(text);
            }
            Node::Element(ElementNode {
                tag_name,
                attributes,
                children,
            }) => {
                self.change_list.emit_create_element(tag_name);
                for attr in attributes {
                    self.change_list.emit_set_attribute(&attr.name, &attr.value);
                }
                for child in children {
                    self.create(child.clone());
                    self.change_list.emit_append_child();
                }
            }
        }
    }
}
