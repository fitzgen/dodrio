use super::change_list::ChangeList;
use super::node::{Node, NodeData, NodeRef};
use super::Render;
use bumpalo::Bump;
use std::mem;

pub struct Vdom {
    dom_buffers: [Bump; 2],
    change_list: ChangeList,

    // Actually a reference into `self.dom_buffers[0]`.
    current_root: Option<NodeRef<'static>>,
}

unsafe fn extend_node_lifetime<'a>(node: NodeRef<'a>) -> NodeRef<'static> {
    mem::transmute(node)
}

impl Vdom {
    pub fn new<R>(container: &web_sys::Node, contents: R) -> Vdom
    where
        R: Render,
    {
        let dom_buffers = [Bump::new(), Bump::new()];
        let change_list = ChangeList::new(container);

        // Create a dummy `<div/>` in our container.
        let current_root: NodeRef = dom_buffers[0].alloc(Node::element("div", [], [])).into();
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

        // Diff and apply the `contents` against this dummy node.
        let mut vdom = Vdom {
            dom_buffers,
            change_list,
            current_root,
        };
        vdom.render(contents);
        vdom
    }

    pub fn render<R>(&mut self, contents: R)
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

    unsafe fn set_current_root(&mut self, current: NodeRef<'static>) {
        debug_assert!(self.current_root.is_none());
        self.current_root = Some(current);
    }

    fn diff(&self, old: NodeRef, new: NodeRef) {
        match (new.data, old.data) {
            (NodeData::Text { text: new_text }, NodeData::Text { text: old_text }) => {
                if new_text != old_text {
                    self.change_list.emit_set_text(new_text);
                }
            }
            (NodeData::Text { .. }, NodeData::Element { .. }) => {
                self.create(new);
                self.change_list.emit_replace_with();
            }
            (NodeData::Element { .. }, NodeData::Text { .. }) => {
                self.create(new);
                self.change_list.emit_replace_with();
            }
            (
                NodeData::Element {
                    tag_name: new_tag_name,
                },
                NodeData::Element {
                    tag_name: old_tag_name,
                },
            ) => {
                if new_tag_name != old_tag_name {
                    self.create(new);
                    self.change_list.emit_replace_with();
                    return;
                }

                // Do O(n^2) passes to add/update and remove attributes, since
                // there are almost always very few attributes.
                'outer: for new_attr in new.attributes {
                    for old_attr in old.attributes {
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
                'outer2: for old_attr in old.attributes {
                    for new_attr in new.attributes {
                        if old_attr.name == new_attr.name {
                            continue 'outer2;
                        }
                    }
                    self.change_list.emit_remove_attribute(old_attr.name);
                }

                let mut new_children = new.children.iter();
                let mut old_children = old.children.iter();
                let mut pushed_first_child = false;

                for (i, (new_child, old_child)) in
                    new_children.by_ref().zip(old_children.by_ref()).enumerate()
                {
                    if i == 0 {
                        self.change_list.emit_push_first_child();
                        pushed_first_child = true;
                    } else {
                        self.change_list.emit_pop_push_next_sibling();
                    }

                    self.diff(old_child.clone(), new_child.clone());
                }

                if old_children.next().is_some() {
                    if !pushed_first_child {
                        self.change_list.emit_push_first_child();
                    }
                    self.change_list.emit_remove_self_and_next_siblings();
                }

                for (i, new_child) in new_children.enumerate() {
                    if i == 0 && pushed_first_child {
                        self.change_list.emit_pop();
                    }
                    self.create(new_child.clone());
                    self.change_list.emit_append_child();
                }

                self.change_list.emit_pop();
            }
        }
    }

    fn create(&self, node: NodeRef) {
        match node.data {
            NodeData::Text { text } => {
                self.change_list.emit_create_text_node(text);
            }
            NodeData::Element { tag_name } => {
                self.change_list.emit_create_element(tag_name);
                for attr in node.attributes {
                    self.change_list.emit_set_attribute(&attr.name, &attr.value);
                }
                for child in node.children {
                    self.create(child.clone());
                    self.change_list.emit_append_child();
                }
            }
        }
    }
}
