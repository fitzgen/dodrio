use std::collections::HashMap;

use crate::{Element, EventsTrampoline};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{window, Document, Event, Node};

#[derive(Debug)]
pub struct ChangeListInterpreter {
    container: Element,
    stack: Vec<Node>,
    strings: HashMap<u32, String>,
    temporaries: Vec<Node>,
    templates: HashMap<u32, Node>,
    callback: Option<Closure<dyn FnMut(&Event)>>,
    document: Document,
}

impl ChangeListInterpreter {
    pub fn new(container: Element) -> Self {
        let document = window()
            .expect("must have access to the window")
            .document()
            .expect("must have access to the Document");

        Self {
            container,
            stack: Vec::with_capacity(5),
            strings: Default::default(),
            temporaries: Default::default(),
            templates: Default::default(),
            callback: None,
            document,
        }
    }

    pub fn unmount(&mut self) {
        self.stack.clear();
        self.strings.clear();
        self.temporaries.clear();
        self.templates.clear();
    }

    pub fn start(&mut self) {
        if let Some(child) = self.container.first_child() {
            self.stack.push(child);
        }
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.temporaries.clear();
    }

    pub fn get_cached_string(&self, id: u32) -> Option<&String> {
        self.strings.get(&id)
    }

    pub fn get_template(&self, id: u32) -> Option<&Node> {
        self.templates.get(&id)
    }

    pub fn init_events_trampoline(&mut self, mut trampoline: EventsTrampoline) {
        self.callback = Some(Closure::wrap(Box::new(move |event: &web_sys::Event| {
            let target = event
                .target()
                .expect("missing target")
                .dyn_into::<Element>()
                .expect("not a valid element");
            let typ = event.type_();
            let a: u32 = target
                .get_attribute(&format!("dodrio-a-{}", typ))
                .and_then(|v| v.parse().ok())
                .unwrap_or_default();

            let b: u32 = target
                .get_attribute(&format!("dodrio-b-{}", typ))
                .and_then(|v| v.parse().ok())
                .unwrap_or_default();

            // get a and b from the target
            trampoline(event.clone(), a, b);
        }) as Box<dyn FnMut(&Event)>));
    }

    /// Get the top value of the stack.
    fn top(&self) -> &Node {
        &self.stack[self.stack.len() - 1]
    }

    // 0
    pub fn set_text(&mut self, text: &str) {
        self.top().set_text_content(Some(text));
    }

    // 1
    pub fn remove_self_and_next_siblings(&mut self) {
        let node = self.stack.pop().unwrap();
        let mut sibling = node.next_sibling();

        while let Some(inner) = sibling {
            let temp = inner.next_sibling();
            if let Some(sibling) = inner.dyn_ref::<Element>() {
                sibling.remove();
            }
            sibling = temp;
        }
        if let Some(node) = node.dyn_ref::<Element>() {
            node.remove();
        }
    }

    // 2
    pub fn replace_with(&mut self) {
        let new_node = self.stack.pop().unwrap();
        let old_node = self.stack.pop().unwrap();
        old_node
            .dyn_ref::<Element>()
            .expect(&format!("not an element: {:?}", old_node))
            .replace_with_with_node_1(&new_node)
            .unwrap();
        self.stack.push(new_node);
    }

    // 3
    pub fn set_attribute(&mut self, name_id: u32, value_id: u32) {
        let name = self.get_cached_string(name_id).unwrap();
        let value = self.get_cached_string(value_id).unwrap();
        let node = self.top();

        if let Some(node) = node.dyn_ref::<Element>() {
            node.set_attribute(name, value).unwrap();

            // Some attributes are "volatile" and don't work through `setAttribute`.
            // TODO:
            // if name == "value" {
            //     node.set_value(value);
            // }
            // if name == "checked" {
            //     node.set_checked(true);
            // }
            // if name == "selected" {
            //     node.set_selected(true);
            // }
        }
    }

    // 4
    pub fn remove_attribute(&mut self, name_id: u32) {
        let name = self.get_cached_string(name_id).unwrap();
        let node = self.top();
        if let Some(node) = node.dyn_ref::<Element>() {
            node.remove_attribute(name).unwrap();

            // Some attributes are "volatile" and don't work through `removeAttribute`.
            // TODO:
            // if name == "value" {
            //     node.set_value("");
            // }
            // if name == "checked" {
            //     node.set_checked(false);
            // }
            // if name == "selected" {
            //     node.set_selected(false);
            // }
        }
    }

    // 5
    pub fn push_reverse_child(&mut self, n: u32) {
        let parent = self.top();
        let children = parent.child_nodes();
        let child = children.get(children.length() - n - 1).unwrap();
        self.stack.push(child);
    }

    // 6
    pub fn pop_push_child(&mut self, n: u32) {
        self.stack.pop();
        let parent = self.top();
        let children = parent.child_nodes();
        let child = children.get(n).unwrap();
        self.stack.push(child);
    }

    // 7
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    // 8
    pub fn append_child(&mut self) {
        let child = self.stack.pop().unwrap();
        self.top().append_child(&child).unwrap();
    }

    // 9
    pub fn create_text_node(&mut self, text: &str) {
        self.stack.push(
            self.document
                .create_text_node(text)
                .dyn_into::<Node>()
                .unwrap(),
        );
    }

    // 10
    pub fn create_element(&mut self, tag_name_id: u32) {
        let tag_name = self.get_cached_string(tag_name_id).unwrap();
        let el = self
            .document
            .create_element(tag_name)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();
        self.stack.push(el);
    }

    // 11
    pub fn new_event_listener(&mut self, event_id: u32, a: u32, b: u32) {
        let event_type = self.get_cached_string(event_id).unwrap();
        if let Some(el) = self.top().dyn_ref::<Element>() {
            el.add_event_listener_with_callback(
                event_type,
                self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
            )
            .unwrap();
            el.set_attribute(&format!("dodrio-a-{}", event_type), &a.to_string())
                .unwrap();
            el.set_attribute(&format!("dodrio-b-{}", event_type), &b.to_string())
                .unwrap();
        }
    }

    // 12
    pub fn update_event_listener(&mut self, event_id: u32, a: u32, b: u32) {
        let event_type = self.get_cached_string(event_id).unwrap();
        if let Some(el) = self.top().dyn_ref::<Element>() {
            el.set_attribute(&format!("dodrio-a-{}", event_type), &a.to_string())
                .unwrap();
            el.set_attribute(&format!("dodrio-b-{}", event_type), &b.to_string())
                .unwrap();
        }
    }

    // 13
    pub fn remove_event_listener(&mut self, event_id: u32) {
        let event_type = self.get_cached_string(event_id).unwrap();
        if let Some(el) = self.top().dyn_ref::<Element>() {
            el.remove_event_listener_with_callback(
                event_type,
                self.callback.as_ref().unwrap().as_ref().unchecked_ref(),
            )
            .unwrap();
        }
    }

    // 14
    pub fn add_cached_string(&mut self, string: &str, id: u32) {
        self.strings.insert(id, string.into());
    }

    // 15
    pub fn drop_cached_string(&mut self, id: u32) {
        self.strings.remove(&id);
    }

    // 16
    pub fn create_element_ns(&mut self, tag_name_id: u32, ns_id: u32) {
        let tag_name = self.get_cached_string(tag_name_id).unwrap();
        let ns = self.get_cached_string(ns_id).unwrap();
        let el = self
            .document
            .create_element_ns(Some(ns), tag_name)
            .unwrap()
            .dyn_into::<Node>()
            .unwrap();
        self.stack.push(el);
    }

    // 17
    pub fn save_children_to_temporaries(&mut self, mut temp: u32, start: u32, end: u32) {
        let parent = self.top();
        let children = parent.child_nodes();
        for i in start..end {
            temp += 1;
            self.temporaries[temp as usize] = children.get(i).unwrap();
        }
    }

    // 18
    pub fn push_child(&mut self, n: u32) {
        let parent = self.top();
        let child = parent.child_nodes().get(n).unwrap();
        self.stack.push(child);
    }

    // 19
    pub fn push_temporary(&mut self, temp: u32) {
        self.stack.push(self.temporaries[temp as usize].clone());
    }

    // 20
    pub fn insert_before(&mut self) {
        let before = self.stack.pop().unwrap();
        let after = self.stack.pop().unwrap();
        after
            .parent_node()
            .unwrap()
            .insert_before(&before, Some(&after))
            .unwrap();
        self.stack.push(before);
    }

    // 21
    pub fn pop_push_reverse_child(&mut self, n: u32) {
        self.stack.pop();
        let parent = self.top();
        let children = parent.child_nodes();
        let child = children.get(children.length() - n - 1).unwrap();
        self.stack.push(child);
    }

    // 22
    pub fn remove_child(&mut self, n: u32) {
        let parent = self.top();
        if let Some(child) = parent.child_nodes().get(n).unwrap().dyn_ref::<Element>() {
            child.remove();
        }
    }

    // 23
    pub fn set_class(&mut self, class_id: u32) {
        let class_name = self.get_cached_string(class_id).unwrap();
        if let Some(el) = self.top().dyn_ref::<Element>() {
            el.set_class_name(class_name);
        }
    }

    // 24
    pub fn save_template(&mut self, id: u32) {
        let template = self.top();
        let t = template.clone_node_with_deep(true).unwrap();
        self.templates.insert(id, t);
    }

    // 25
    pub fn push_template(&mut self, id: u32) {
        let template = self.get_template(id).unwrap();
        let t = template.clone_node_with_deep(true).unwrap();
        self.stack.push(t);
    }
}
