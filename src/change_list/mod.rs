pub(crate) mod emitter;
pub(crate) mod strings;

// Note: has to be `pub` because of `wasm-bindgen` visibility restrictions.
pub mod js;

use self::emitter::InstructionEmitter;
use self::strings::{StringKey, StringsCache};
use crate::Listener;

#[derive(Debug)]
pub(crate) struct ChangeListPersistentState {
    strings: StringsCache,
    emitter: InstructionEmitter,
    interpreter: js::ChangeListInterpreter,
}

pub(crate) struct ChangeListBuilder<'a> {
    state: &'a mut ChangeListPersistentState,
    next_temporary: u32,
}

impl Drop for ChangeListPersistentState {
    fn drop(&mut self) {
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

    pub(crate) fn builder<'a>(&'a mut self) -> ChangeListBuilder<'a> {
        ChangeListBuilder {
            state: self,
            next_temporary: 0,
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
    pub fn next_temporary(&self) -> u32 {
        self.next_temporary
    }

    pub fn set_next_temporary(&mut self, next_temporary: u32) {
        self.next_temporary = next_temporary;
    }

    pub fn save_children_to_temporaries(&mut self, start: usize, end: usize) -> u32 {
        debug_assert!(start < end);
        let temp_base = self.next_temporary;
        debug!(
            "emit: save_children_to_temporaries({}, {}, {})",
            temp_base, start, end
        );
        self.next_temporary = temp_base + (end - start) as u32;
        self.state
            .emitter
            .save_children_to_temporaries(temp_base, start as u32, end as u32);
        temp_base
    }

    pub fn push_temporary(&self, temp: u32) {
        debug!("emit: push_temporary({})", temp);
        self.state.emitter.push_temporary(temp);
    }

    pub fn push_child(&self, child: usize) {
        debug!("emit: push_child({})", child);
        self.state.emitter.push_child(child as u32);
    }

    pub fn remove_child(&self, child: usize) {
        debug!("emit: remove_child({})", child);
        self.state.emitter.remove_child(child as u32);
    }

    pub fn push_last_child(&self) {
        debug!("emit: push_last_child()");
        self.state.emitter.push_last_child();
    }

    pub fn insert_before(&self) {
        debug!("emit: insert_before()");
        self.state.emitter.insert_before();
    }

    pub fn ensure_string(&mut self, string: &str) -> StringKey {
        self.state
            .strings
            .ensure_string(string, &self.state.emitter)
    }

    pub fn set_text(&self, text: &str) {
        debug!("emit: set_text({:?})", text);
        self.state
            .emitter
            .set_text(text.as_ptr() as u32, text.len() as u32);
    }

    pub fn remove_self_and_next_siblings(&self) {
        debug!("emit: remove_self_and_next_siblings()");
        self.state.emitter.remove_self_and_next_siblings();
    }

    pub fn replace_with(&self) {
        debug!("emit: replace_with()");
        self.state.emitter.replace_with();
    }

    pub fn set_attribute(&mut self, name: &str, value: &str) {
        debug!("emit: set_attribute({:?}, {:?})", name, value);
        let name_id = self.ensure_string(name);
        let value_id = self.ensure_string(value);
        self.state
            .emitter
            .set_attribute(name_id.into(), value_id.into());
    }

    pub fn remove_attribute(&mut self, name: &str) {
        debug!("emit: remove_attribute({:?})", name);
        let name_id = self.ensure_string(name);
        self.state.emitter.remove_attribute(name_id.into());
    }

    pub fn push_first_child(&self) {
        debug!("emit: push_first_child");
        self.state.emitter.push_first_child();
    }

    pub fn pop_push_next_sibling(&self) {
        debug!("emit: pop_push_next_sibling");
        self.state.emitter.pop_push_next_sibling();
    }

    pub fn pop(&self) {
        debug!("emit: pop");
        self.state.emitter.pop();
    }

    pub fn append_child(&self) {
        debug!("emit: append_child()");
        self.state.emitter.append_child();
    }

    pub fn create_text_node(&self, text: &str) {
        debug!("emit: create_text_node({:?})", text);
        self.state
            .emitter
            .create_text_node(text.as_ptr() as u32, text.len() as u32);
    }

    pub fn create_element(&mut self, tag_name: &str) {
        debug!("emit: create_element({:?})", tag_name);
        let tag_name_id = self.ensure_string(tag_name);
        self.state.emitter.create_element(tag_name_id.into());
    }

    pub fn create_element_ns(&mut self, tag_name: &str, ns: &str) {
        debug!("emit: create_element_ns({:?}, {:?})", tag_name, ns);
        let tag_name_id = self.ensure_string(tag_name);
        let ns_id = self.ensure_string(ns);
        self.state
            .emitter
            .create_element_ns(tag_name_id.into(), ns_id.into());
    }

    pub fn new_event_listener(&mut self, listener: &Listener) {
        debug!("emit: new_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.state.emitter.new_event_listener(event_id.into(), a, b);
    }

    pub fn update_event_listener(&mut self, listener: &Listener) {
        debug!("emit: update_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.state
            .emitter
            .update_event_listener(event_id.into(), a, b);
    }

    pub fn remove_event_listener(&mut self, event: &str) {
        debug!("emit: remove_event_listener({:?})", event);
        let event_id = self.ensure_string(event);
        self.state.emitter.remove_event_listener(event_id.into());
    }
}
