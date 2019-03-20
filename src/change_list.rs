use crate::Listener;
use bumpalo::Bump;
use fxhash::FxHashMap;
use std::fmt;

pub mod js {
    cfg_if::cfg_if! {
        if #[cfg(feature = "xxx-unstable-internal-use-only")] {
            #[derive(Clone, Debug)]
            pub struct ChangeList {}
            impl ChangeList {
                pub fn new(_container: &crate::Element) -> ChangeList {
                    ChangeList {}
                }
                pub fn unmount(&self) {}
                pub fn add_change_list_range(&self, _start: usize, _len: usize) {}
                pub fn init_events_trampoline(&self, _trampoline: &crate::EventsTrampoline) {}
            }

            pub fn eval_change_list() {}
        } else {
            use wasm_bindgen::prelude::*;

            #[wasm_bindgen]
            extern "C" {
                #[derive(Clone, Debug)]
                pub type ChangeList;

                #[wasm_bindgen(constructor)]
                pub fn new(container: &web_sys::Element) -> ChangeList;

                #[wasm_bindgen(structural, method)]
                pub fn unmount(this: &ChangeList);

                #[wasm_bindgen(structural, method, js_name = addChangeListRange)]
                pub fn add_change_list_range(this: &ChangeList, start: usize, len: usize);

                #[wasm_bindgen(structural, method, js_name = applyChanges)]
                pub fn apply_changes(this: &ChangeList, memory: JsValue);

                #[wasm_bindgen(structural, method, js_name = initEventsTrampoline)]
                pub fn init_events_trampoline(
                    this: &ChangeList,
                    trampoline: &crate::EventsTrampoline,
                );
            }

            pub fn eval_change_list() {
                use std::sync::Once;
                // XXX: Because wasm-bindgen-test doesn't support third party JS
                // dependencies, we can't use `wasm_bindgen(module = "...")` for our
                // `ChangeList` JS import. Instead, this *should* be a local JS snippet,
                // but that isn't implemented yet:
                // https://github.com/rustwasm/rfcs/pull/6
                static EVAL: Once = Once::new();
                EVAL.call_once(|| {
                    js_sys::eval(include_str!("../js/change-list.js"))
                        .expect_throw("should eval change-list.js OK");
                });
            }
        }
    }
}

struct StringsCacheEntry {
    key: u32,
    used: bool,
}

pub(crate) struct ChangeList {
    bump: Bump,
    strings_cache: FxHashMap<String, StringsCacheEntry>,
    next_string_key: u32,
    js: js::ChangeList,
    events_trampoline: Option<crate::EventsTrampoline>,
}

impl Drop for ChangeList {
    fn drop(&mut self) {
        debug!("Dropping ChangeList");
        self.js.unmount();
    }
}

impl fmt::Debug for ChangeList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ChangeList")
            .field("bump", &self.bump)
            .field("js", &self.js)
            .field("events_trampoline", &"..")
            .finish()
    }
}

impl ChangeList {
    pub(crate) fn new(container: &crate::Element) -> ChangeList {
        js::eval_change_list();
        let bump = Bump::new();
        let strings_cache = FxHashMap::default();
        let js = js::ChangeList::new(container);
        ChangeList {
            bump,
            strings_cache,
            next_string_key: 0,
            js,
            events_trampoline: None,
        }
    }

    #[cfg(feature = "xxx-unstable-internal-use-only")]
    pub(crate) fn apply_changes(&mut self) {
        // Do nothing...
    }

    #[cfg(not(feature = "xxx-unstable-internal-use-only"))]
    pub(crate) fn apply_changes(&mut self) {
        let js = &self.js;
        unsafe {
            self.bump.each_allocated_chunk(|ch| {
                js.add_change_list_range(ch.as_ptr() as usize, ch.len());
            });
        }
        js.apply_changes(wasm_bindgen::memory());
        self.bump.reset();
    }

    pub(crate) fn init_events_trampoline(&mut self, trampoline: crate::EventsTrampoline) {
        debug_assert!(self.events_trampoline.is_none());
        self.js.init_events_trampoline(&trampoline);
        self.events_trampoline = Some(trampoline);
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
enum ChangeDiscriminant {
    /// Immediates: `(pointer, length)`
    ///
    /// Stack: `[... TextNode] -> [... TextNode]`
    ///
    /// ```text
    /// stack.top().textContent = String(pointer, length)
    /// ```
    SetText = 0,

    /// Immediates: `()`
    ///
    /// Stack: `[... Node] -> [...]`
    ///
    /// ```text
    /// node = stack.pop()
    /// while (node.nextSibling) {
    ///   node.nextSibling.remove();
    /// }
    /// node.remove()
    /// ```
    RemoveSelfAndNextSiblings = 1,

    /// Immediates: `()`
    ///
    /// Stack: `[... Node Node] -> [... Node]`
    ///
    /// ```text
    /// new = stack.pop()
    /// old = stack.pop()
    /// old.replaceWith(new)
    /// stack.push(new)
    /// ```
    ReplaceWith = 2,

    /// Immediates: `(id1, id2)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().setAttribute(String(id1), String(id2))
    /// ```
    SetAttribute = 3,

    /// Immediates: (id)
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().removeAttribute(String(id))
    /// ```
    RemoveAttribute = 4,

    /// Immediates: `()`
    ///
    /// Stack: `[... Node] -> [... Node node]`
    ///
    /// ```text
    /// stack.push(stack.top().firstChild)
    /// ```
    PushFirstChild = 5,

    /// Immediates: `()`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.push(stack.pop().nextSibling)
    /// ```
    PopPushNextSibling = 6,

    /// Immediates: `()`
    ///
    /// Stack: `[... T] -> [...]`
    ///
    /// ```text
    /// stack.pop()
    /// ```
    Pop = 7,

    /// Immediates: `()`
    ///
    /// Stack: `[... Node Node] -> [... Node]`
    ///
    /// ```text
    /// child = stack.pop()
    /// stack.top().appendChild(child)
    /// ```
    AppendChild = 8,

    /// Immediates: `(pointer, length)`
    ///
    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(createTextNode(String(pointer, length)))
    /// ```
    CreateTextNode = 9,

    /// Immediates: `(id)`
    ///
    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(createElement(String(id))
    /// ```
    CreateElement = 10,

    /// Immediates: `(id, A, B)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// event = String(id)
    /// callback = ProxyToRustCallback(A, B)
    /// stack.top().addEventListener(event, callback)
    /// ```
    NewEventListener = 11,

    /// Immediates: `(id, A, B)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// event = String(id)
    /// new_callback = ProxyToRustCallback(A, B);
    /// stack.top().updateEventlistener(new_callback)
    /// ```
    UpdateEventListener = 12,

    /// Immediates: `(id)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().removeEventListener(String(id));
    /// ```
    RemoveEventListener = 13,

    /// Immediates: `(pointer, length, id)`
    ///
    /// Stack: `[...] -> [...]`
    ///
    /// ```text
    /// addString(String(pointer, length), id);
    /// ```
    AddString = 14,

    /// Immediates: `(id)`
    ///
    /// Stack: `[...] -> [...]`
    ///
    /// ```text
    /// dropString(id);
    /// ```
    DropString = 15,

    /// Immediates: `(id1, id2)`
    ///
    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(createElementNS(String(id1), String(id2))
    /// ```
    CreateElementNS = 16,

    /// Immediates: `(id1, id2, id3)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().setAttributeNS(String(id1), String(id2), String(id3))
    /// ```
    SetAttributeNS = 17,
}

// Allocation utilities to ensure that we only allocation sequences of `u32`s
// into the change list's bump arena without any padding. This helps maintain
// the invariants required for `Bump::each_allocated_chunk`'s safety.
impl ChangeList {
    // Allocate an opcode with zero immediates.
    fn op0(&self, discriminant: ChangeDiscriminant) {
        self.bump.alloc(discriminant as u32);
    }

    // Allocate an opcode with one immediate.
    fn op1(&self, discriminant: ChangeDiscriminant, a: u32) {
        self.bump.alloc([discriminant as u32, a]);
    }

    // Allocate an opcode with two immediates.
    fn op2(&self, discriminant: ChangeDiscriminant, a: u32, b: u32) {
        self.bump.alloc([discriminant as u32, a, b]);
    }

    // Allocate an opcode with three immediates.
    fn op3(&self, discriminant: ChangeDiscriminant, a: u32, b: u32, c: u32) {
        self.bump.alloc([discriminant as u32, a, b, c]);
    }

    // Note: no 4-immediate opcodes at this time.
}

impl ChangeList {
    fn ensure_string(&mut self, string: &str) -> u32 {
        if let Some(entry) = self.strings_cache.get_mut(string) {
            entry.used = true;
            entry.key
        } else {
            let key = self.next_string_key;
            self.next_string_key += 1;
            let entry = StringsCacheEntry { key, used: true };
            self.strings_cache.insert(string.to_string(), entry);
            self.op3(
                ChangeDiscriminant::AddString,
                string.as_ptr() as u32,
                string.len() as u32,
                key,
            );
            key
        }
    }

    pub(crate) fn drop_unused_strings(&mut self) {
        debug!("drop_unused_strings()");
        let mut new_cache = FxHashMap::default();
        for (key, val) in self.strings_cache.iter() {
            if val.used {
                new_cache.insert(
                    key.clone(),
                    StringsCacheEntry {
                        key: val.key,
                        used: false,
                    },
                );
            } else {
                self.op1(ChangeDiscriminant::DropString, val.key);
            }
        }
        self.strings_cache = new_cache;
    }

    pub(crate) fn emit_set_text(&self, text: &str) {
        debug!("emit_set_text({:?})", text);
        self.op2(
            ChangeDiscriminant::SetText,
            text.as_ptr() as u32,
            text.len() as u32,
        );
    }

    pub(crate) fn emit_remove_self_and_next_siblings(&self) {
        debug!("emit_remove_self_and_next_siblings()");
        self.op0(ChangeDiscriminant::RemoveSelfAndNextSiblings);
    }

    pub(crate) fn emit_replace_with(&self) {
        debug!("emit_replace_with()");
        self.op0(ChangeDiscriminant::ReplaceWith);
    }

    pub(crate) fn emit_set_attribute(&mut self, name: &str, value: &str) {
        debug!("emit_set_attribute({:?}, {:?})", name, value);
        let name_id = self.ensure_string(name);
        let value_id = self.ensure_string(value);
        self.op2(ChangeDiscriminant::SetAttribute, name_id, value_id);
    }

    pub(crate) fn emit_set_attribute_ns(&mut self, name: &str, value: &str) {
        debug!("emit_set_attribute_ns({:?}, {:?}", name, value);
        let name_id = self.ensure_string(name);
        let value_id = self.ensure_string(value);
        self.op2(ChangeDiscriminant::SetAttributeNS, name_id, value_id);
    }

    pub(crate) fn emit_remove_attribute(&mut self, name: &str) {
        debug!("emit_remove_attribute({:?})", name);
        let name_id = self.ensure_string(name);
        self.op1(ChangeDiscriminant::RemoveAttribute, name_id);
    }

    pub(crate) fn emit_push_first_child(&self) {
        debug!("emit_push_first_child()");
        self.op0(ChangeDiscriminant::PushFirstChild);
    }

    pub(crate) fn emit_pop_push_next_sibling(&self) {
        debug!("emit_pop_push_next_sibling()");
        self.op0(ChangeDiscriminant::PopPushNextSibling);
    }

    pub(crate) fn emit_pop(&self) {
        debug!("emit_pop()");
        self.op0(ChangeDiscriminant::Pop);
    }

    pub(crate) fn emit_append_child(&self) {
        debug!("emit_append_child()");
        self.op0(ChangeDiscriminant::AppendChild);
    }

    pub(crate) fn emit_create_text_node(&self, text: &str) {
        debug!("emit_create_text_node({:?})", text);
        self.op2(
            ChangeDiscriminant::CreateTextNode,
            text.as_ptr() as u32,
            text.len() as u32,
        );
    }

    pub(crate) fn emit_create_element(&mut self, tag_name: &str) {
        debug!("emit_create_element({:?})", tag_name);
        let tag_name_id = self.ensure_string(tag_name);
        self.op1(ChangeDiscriminant::CreateElement, tag_name_id);
    }

    pub(crate) fn emit_create_element_ns(&mut self, tag_name: &str, ns: &str) {
        debug!("emit_create_element_ns({:?}, {:?})", tag_name, ns);
        let tag_name_id = self.ensure_string(tag_name);
        let ns_id = self.ensure_string(ns);
        self.op2(ChangeDiscriminant::CreateElementNS, tag_name_id, ns_id);
    }

    pub(crate) fn emit_new_event_listener(&mut self, listener: &Listener) {
        debug!("emit_new_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.op3(ChangeDiscriminant::NewEventListener, event_id, a, b);
    }

    pub(crate) fn emit_update_event_listener(&mut self, listener: &Listener) {
        debug!("emit_update_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        let event_id = self.ensure_string(listener.event);
        self.op3(ChangeDiscriminant::UpdateEventListener, event_id, a, b);
    }

    pub(crate) fn emit_remove_event_listener(&mut self, event: &str) {
        debug!("emit_remove_event_listener({:?})", event);
        let event_id = self.ensure_string(event);
        self.op1(ChangeDiscriminant::RemoveEventListener, event_id);
    }
}
