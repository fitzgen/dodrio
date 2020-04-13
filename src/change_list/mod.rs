pub(crate) mod interpreter;
pub(crate) mod strings;
pub(crate) mod traversal;

// Note: has to be `pub` because of `wasm-bindgen` visibility restrictions.
pub mod js;

use self::interpreter::ChangeListInterpreter;
use self::strings::{StringKey, StringsCache};
use self::traversal::{MoveTo, Traversal};
use crate::{cached_set::CacheId, Listener};
use fxhash::FxHashSet;

#[derive(Debug)]
pub(crate) struct ChangeListPersistentState {
    strings: StringsCache,
    traversal: Traversal,
    interpreter: ChangeListInterpreter,
    templates: FxHashSet<CacheId>,
}

pub(crate) struct ChangeListBuilder<'a> {
    state: &'a mut ChangeListPersistentState,
    next_temporary: u32,
    forcing_new_listeners: bool,
}

impl Drop for ChangeListPersistentState {
    fn drop(&mut self) {
        self.interpreter.unmount();
    }
}

impl ChangeListPersistentState {
    pub(crate) fn new(container: &crate::Element) -> ChangeListPersistentState {
        let strings = StringsCache::new();
        let traversal = Traversal::new();
        let interpreter = ChangeListInterpreter::new(container.clone());
        let templates = Default::default();

        ChangeListPersistentState {
            strings,
            traversal,
            interpreter,
            templates,
        }
    }

    pub(crate) fn init_events_trampoline(&mut self, trampoline: crate::EventsTrampoline) {
        self.interpreter.init_events_trampoline(trampoline);
    }

    pub(crate) fn builder(&mut self) -> ChangeListBuilder {
        let builder = ChangeListBuilder {
            state: self,
            next_temporary: 0,
            forcing_new_listeners: false,
        };
        debug!("emit: start");
        builder.state.interpreter.start();

        builder
    }
}

impl ChangeListBuilder<'_> {
    pub(crate) fn finish(self) {
        self.state
            .strings
            .drop_unused_strings(&mut self.state.interpreter);

        debug!("emit: reset");
        self.state.interpreter.reset();
        self.state.traversal.reset();
    }

    /// Traversal methods.

    pub fn go_down(&mut self) {
        self.state.traversal.down();
    }

    pub fn go_down_to_child(&mut self, index: usize) {
        self.state.traversal.down();
        self.state.traversal.sibling(index);
    }

    pub fn go_down_to_reverse_child(&mut self, index: usize) {
        self.state.traversal.down();
        self.state.traversal.reverse_sibling(index);
    }

    pub fn go_up(&mut self) {
        self.state.traversal.up();
    }

    pub fn go_to_sibling(&mut self, index: usize) {
        self.state.traversal.sibling(index);
    }

    pub fn go_to_temp_sibling(&mut self, temp: u32) {
        self.state.traversal.up();
        self.state.traversal.down_to_temp(temp);
    }

    pub fn go_down_to_temp_child(&mut self, temp: u32) {
        self.state.traversal.down_to_temp(temp);
    }

    pub fn commit_traversal(&mut self) {
        if self.state.traversal.is_committed() {
            return;
        }

        for mv in self.state.traversal.commit() {
            match mv {
                MoveTo::Parent => {
                    debug!("emit: pop");
                    self.state.interpreter.pop();
                }
                MoveTo::Child(n) => {
                    debug!("emit: push_child({})", n);
                    self.state.interpreter.push_child(n);
                }
                MoveTo::ReverseChild(n) => {
                    debug!("emit: push_reverse_child({})", n);
                    self.state.interpreter.push_reverse_child(n);
                }
                MoveTo::Sibling(n) => {
                    debug!("emit: pop_push_child({})", n);
                    self.state.interpreter.pop_push_child(n);
                }
                MoveTo::ReverseSibling(n) => {
                    debug!("emit: pop_push_reverse_child({})", n);
                    self.state.interpreter.pop_push_reverse_child(n);
                }
                MoveTo::TempChild(temp) => {
                    debug!("emit: push_temporary({})", temp);
                    self.state.interpreter.push_temporary(temp);
                }
            }
        }
    }

    pub fn traversal_is_committed(&self) -> bool {
        self.state.traversal.is_committed()
    }
}

impl ChangeListBuilder<'_> {
    pub fn next_temporary(&self) -> u32 {
        self.next_temporary
    }

    pub fn set_next_temporary(&mut self, next_temporary: u32) {
        self.next_temporary = next_temporary;
    }

    pub fn save_children_to_temporaries(&mut self, start: usize, end: usize) -> u32 {
        debug_assert!(self.traversal_is_committed());
        debug_assert!(start < end);
        let temp_base = self.next_temporary;
        debug!(
            "emit: save_children_to_temporaries({}, {}, {})",
            temp_base, start, end
        );
        self.next_temporary = temp_base + (end - start) as u32;
        self.state
            .interpreter
            .save_children_to_temporaries(temp_base, start as u32, end as u32);
        temp_base
    }

    pub fn push_temporary(&mut self, temp: u32) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: push_temporary({})", temp);
        self.state.interpreter.push_temporary(temp);
    }

    pub fn remove_child(&mut self, child: usize) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: remove_child({})", child);
        self.state.interpreter.remove_child(child as u32);
    }

    pub fn insert_before(&mut self) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: insert_before()");
        self.state.interpreter.insert_before();
    }

    pub fn ensure_string(&mut self, string: &str) -> StringKey {
        self.state
            .strings
            .ensure_string(string, &mut self.state.interpreter)
    }

    pub fn set_text(&mut self, text: &str) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: set_text({:?})", text);
        self.state.interpreter.set_text(text);
    }

    pub fn remove_self_and_next_siblings(&mut self) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: remove_self_and_next_siblings()");
        self.state.interpreter.remove_self_and_next_siblings();
    }

    pub fn replace_with(&mut self) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: replace_with()");
        self.state.interpreter.replace_with();
    }

    pub fn set_attribute(&mut self, name: &str, value: &str, is_namespaced: bool) {
        debug_assert!(self.traversal_is_committed());
        if name == "class" && !is_namespaced {
            let class_id = self.ensure_string(value);
            debug!("emit: set_class({:?})", value);
            self.state.interpreter.set_class(class_id.into());
        } else {
            let name_id = self.ensure_string(name);
            let value_id = self.ensure_string(value);
            debug!("emit: set_attribute({:?}, {:?})", name, value);
            self.state
                .interpreter
                .set_attribute(name_id.into(), value_id.into());
        }
    }

    pub fn remove_attribute(&mut self, name: &str) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: remove_attribute({:?})", name);
        let name_id = self.ensure_string(name);
        self.state.interpreter.remove_attribute(name_id.into());
    }

    pub fn append_child(&mut self) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: append_child()");
        self.state.interpreter.append_child();
    }

    pub fn create_text_node(&mut self, text: &str) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: create_text_node({:?})", text);
        self.state.interpreter.create_text_node(text);
    }

    pub fn create_element(&mut self, tag_name: &str) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: create_element({:?})", tag_name);
        let tag_name_id = self.ensure_string(tag_name);
        self.state.interpreter.create_element(tag_name_id.into());
    }

    pub fn create_element_ns(&mut self, tag_name: &str, ns: &str) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: create_element_ns({:?}, {:?})", tag_name, ns);
        let tag_name_id = self.ensure_string(tag_name);
        let ns_id = self.ensure_string(ns);
        self.state
            .interpreter
            .create_element_ns(tag_name_id.into(), ns_id.into());
    }

    pub fn push_force_new_listeners(&mut self) -> bool {
        let old = self.forcing_new_listeners;
        self.forcing_new_listeners = true;
        old
    }

    pub fn pop_force_new_listeners(&mut self, previous: bool) {
        debug_assert!(self.forcing_new_listeners);
        self.forcing_new_listeners = previous;
    }

    pub fn new_event_listener(&mut self, listener: &Listener) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: new_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.state
            .interpreter
            .new_event_listener(event_id.into(), a, b);
    }

    pub fn update_event_listener(&mut self, listener: &Listener) {
        debug_assert!(self.traversal_is_committed());

        if self.forcing_new_listeners {
            self.new_event_listener(listener);
            return;
        }

        debug!("emit: update_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.state
            .interpreter
            .update_event_listener(event_id.into(), a, b);
    }

    pub fn remove_event_listener(&mut self, event: &str) {
        debug_assert!(self.traversal_is_committed());
        debug!("emit: remove_event_listener({:?})", event);
        let event_id = self.ensure_string(event);
        self.state
            .interpreter
            .remove_event_listener(event_id.into());
    }

    #[inline]
    pub fn has_template(&mut self, id: CacheId) -> bool {
        self.state.templates.contains(&id)
    }

    pub fn save_template(&mut self, id: CacheId) {
        debug_assert!(self.traversal_is_committed());
        debug_assert!(!self.has_template(id));
        debug!("emit: save_template({:?})", id);
        self.state.templates.insert(id);
        self.state.interpreter.save_template(id.into());
    }

    pub fn push_template(&mut self, id: CacheId) {
        debug_assert!(self.traversal_is_committed());
        debug_assert!(self.has_template(id));
        debug!("emit: push_template({:?})", id);
        self.state.interpreter.push_template(id.into());
    }
}
