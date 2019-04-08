pub(crate) mod emitter;
pub(crate) mod strings;

// Note: has to be `pub` because of `wasm-bindgen` visibility restrictions.
pub mod js;

use self::emitter::InstructionEmitter;
use self::strings::{StringKey, StringsCache};
use crate::traversal::{MoveTo, Traversal};
use crate::Listener;
use bumpalo::Bump;

#[derive(Debug)]
pub(crate) struct ChangeListPersistentState {
    strings: StringsCache,
    emitter: InstructionEmitter,
    interpreter: js::ChangeListInterpreter,
}

pub(crate) struct ChangeListBuilder<'a> {
    state: &'a mut ChangeListPersistentState,
    traversal: Traversal<'a>,
}

impl Drop for ChangeListPersistentState {
    fn drop(&mut self) {
        debug!("Dropping ChangeListPersistentState");
        self.interpreter.unmount();
    }
}

impl ChangeListPersistentState {
    pub(crate) fn new(container: &crate::Element) -> ChangeListPersistentState {
        let strings = StringsCache::new();
        let emitter = InstructionEmitter::new();
        let interpreter = js::ChangeListInterpreter::new(container);
        ChangeListPersistentState {
            strings,
            emitter,
            interpreter,
        }
    }

    pub(crate) fn init_events_trampoline(&mut self, trampoline: &crate::EventsTrampoline) {
        self.interpreter.init_events_trampoline(trampoline);
    }

    pub(crate) fn builder<'a>(&'a mut self, bump: &'a Bump) -> ChangeListBuilder<'a> {
        ChangeListBuilder {
            state: self,
            traversal: Traversal::new(bump),
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(all(feature = "xxx-unstable-internal-use-only", not(target_arch = "wasm32")))] {
        impl ChangeListBuilder<'_> {
            pub(crate) fn finish(self) {
                self.state.strings.drop_unused_strings(&self.state.emitter);

                // Nothing to actually apply the changes to.

                self.state.emitter.reset();
            }
        }
    } else {
        impl ChangeListBuilder<'_> {
            pub(crate) fn finish(self) {
                self.state.strings.drop_unused_strings(&self.state.emitter);

                // Apply the changes.
                let interpreter = &self.state.interpreter;
                self.state.emitter.each_instruction_sequence(|seq| {
                    interpreter.add_change_list_range(seq.as_ptr() as usize, seq.len());
                });
                interpreter.apply_changes(wasm_bindgen::memory());

                self.state.emitter.reset();
            }
        }
    }
}

impl ChangeListBuilder<'_> {
    #[inline]
    pub fn up(&mut self) {
        self.traversal.up();
    }

    #[inline]
    pub fn down(&mut self) {
        self.traversal.down();
    }

    #[inline]
    pub fn forward(&mut self) {
        self.traversal.forward();
    }

    #[inline]
    pub fn traversal_is_committed(&self) -> bool {
        self.traversal.is_committed()
    }

    #[inline]
    pub fn commit_traversal(&mut self) {
        debug!("ChangeListBuilder::commit_traversal");
        if self.traversal.is_committed() {
            return;
        }

        self.do_commit_traversal();
    }

    fn do_commit_traversal(&mut self) {
        for mv in self.traversal.commit() {
            debug!("do_commit_traversal: {:?}", mv);
            match mv {
                MoveTo::Parent => {
                    debug!("emit: pop()");
                    self.state.emitter.pop();
                }
                MoveTo::Child(0) => {
                    debug!("emit: push_first_child()");
                    self.state.emitter.push_first_child();
                }
                MoveTo::Child(i) => {
                    debug!("emit: push_child({})", i);
                    self.state.emitter.push_child(i);
                }
                MoveTo::Sibling(1) => {
                    debug!("emit: pop_push_next_sibling()");
                    self.state.emitter.pop_push_next_sibling();
                }
                MoveTo::Sibling(i) => {
                    debug_assert_ne!(i, 0);
                    debug!("emit: pop_push_sibling({})", i);
                    self.state.emitter.pop_push_sibling(i);
                }
            }
        }
    }

    pub fn ensure_string(&mut self, string: &str) -> StringKey {
        self.state
            .strings
            .ensure_string(string, &self.state.emitter)
    }

    pub fn set_text(&self, text: &str) {
        debug!("emit: set_text({:?})", text);
        debug_assert!(self.traversal.is_committed());
        self.state
            .emitter
            .set_text(text.as_ptr() as u32, text.len() as u32);
    }

    pub fn remove_self_and_next_siblings(&self) {
        debug!("emit: remove_self_and_next_siblings()");
        debug_assert!(self.traversal.is_committed());
        self.state.emitter.remove_self_and_next_siblings();
    }

    pub fn replace_with(&self) {
        debug!("emit: replace_with()");
        debug_assert!(self.traversal.is_committed());
        self.state.emitter.replace_with();
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        debug!("emit: set_attribute({:?}, {:?})", name, value);
        debug_assert!(self.traversal.is_committed());
        let name_id = self.ensure_string(name);
        let value_id = self.ensure_string(value);
        self.state
            .emitter
            .set_attribute(name_id.into(), value_id.into());
    }

    pub fn remove_attribute(&mut self, name: &str) {
        debug!("emit: remove_attribute({:?})", name);
        debug_assert!(self.traversal.is_committed());
        let name_id = self.ensure_string(name);
        self.state.emitter.remove_attribute(name_id.into());
    }

    pub fn append_child(&self) {
        debug!("emit: append_child()");
        debug_assert!(self.traversal.is_committed());
        self.state.emitter.append_child();
    }

    pub fn create_text_node(&self, text: &str) {
        debug!("emit: create_text_node({:?})", text);
        debug_assert!(self.traversal.is_committed());
        self.state
            .emitter
            .create_text_node(text.as_ptr() as u32, text.len() as u32);
    }

    pub fn create_element(&mut self, tag_name: &str) {
        debug!("emit: create_element({:?})", tag_name);
        debug_assert!(self.traversal.is_committed());
        let tag_name_id = self.ensure_string(tag_name);
        self.state.emitter.create_element(tag_name_id.into());
    }

    pub fn create_element_ns(&mut self, tag_name: &str, ns: &str) {
        debug!("emit: create_element_ns({:?}, {:?})", tag_name, ns);
        debug_assert!(self.traversal.is_committed());
        let tag_name_id = self.ensure_string(tag_name);
        let ns_id = self.ensure_string(ns);
        self.state
            .emitter
            .create_element_ns(tag_name_id.into(), ns_id.into());
    }

    pub fn new_event_listener(&mut self, listener: &Listener) {
        debug!("emit: new_event_listener({:?})", listener);
        debug_assert!(self.traversal.is_committed());
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.state.emitter.new_event_listener(event_id.into(), a, b);
    }

    pub fn update_event_listener(&mut self, listener: &Listener) {
        debug!("emit: update_event_listener({:?})", listener);
        debug_assert!(self.traversal.is_committed());
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.state
            .emitter
            .update_event_listener(event_id.into(), a, b);
    }

    pub fn remove_event_listener(&mut self, event: &str) {
        debug!("emit: remove_event_listener({:?})", event);
        debug_assert!(self.traversal.is_committed());
        let event_id = self.ensure_string(event);
        self.state.emitter.remove_event_listener(event_id.into());
    }
}
