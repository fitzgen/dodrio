use crate::Listener;
use bumpalo::Bump;
use log::*;
use std::fmt;
use std::sync::Once;
use wasm_bindgen::prelude::*;

pub mod js {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[derive(Clone, Debug)]
        pub type ChangeList;

        #[wasm_bindgen(constructor)]
        pub fn new(container: &web_sys::Node) -> ChangeList;

        #[wasm_bindgen(structural, method)]
        pub fn unmount(this: &ChangeList);

        #[wasm_bindgen(structural, method, js_name = addChangeListRange)]
        pub fn add_change_list_range(this: &ChangeList, start: usize, len: usize);

        #[wasm_bindgen(structural, method, js_name = applyChanges)]
        pub fn apply_changes(this: &ChangeList, memory: JsValue);

        #[wasm_bindgen(structural, method, js_name = initEventsTrampoline)]
        pub fn init_events_trampoline(
            this: &ChangeList,
            trampoline: &Closure<Fn(web_sys::Event, u32, u32)>,
        );
    }
}

pub(crate) struct ChangeList {
    bump: Bump,
    js: js::ChangeList,
    events_trampoline: Option<Closure<Fn(web_sys::Event, u32, u32)>>,
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
    pub(crate) fn new(container: &web_sys::Node) -> ChangeList {
        // XXX: Because wasm-bindgen-test doesn't support third party JS
        // dependencies, we can't use `wasm_bindgen(module = "...")` for our
        // `ChangeList` JS import. Instead, this *should* be a local JS snippet,
        // but that isn't implemented yet:
        // https://github.com/rustwasm/rfcs/pull/6
        static EVAL: Once = Once::new();
        EVAL.call_once(|| {
            js_sys::eval(include_str!("../js/change-list.js"))
                .expect("should eval change-list.js OK");
        });

        let bump = Bump::new();
        let js = js::ChangeList::new(container);
        ChangeList {
            bump,
            js,
            events_trampoline: None,
        }
    }

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

    pub(crate) fn init_events_trampoline(
        &mut self,
        trampoline: Closure<Fn(web_sys::Event, u32, u32)>,
    ) {
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

    /// Immediates: `(pointer1, length1, pointer2, length2)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().setAttribute(String(pointer1, length1), String(pointer2, length2))
    /// ```
    SetAttribute = 3,

    /// Immediates: (pointer, length)
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().removeAttribute(String(pointer, length))
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

    /// Immediates: `(pointer, length)`
    ///
    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(createElement(String(pointer, length))
    /// ```
    CreateElement = 10,

    /// Immediate: `(pointer, length, A, B)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// event = String(pointer, length)
    /// callback = ProxyToRustCallback(A, B);
    /// stack.top().addEventListener(event, callback);
    /// ```
    NewEventListener = 11,

    /// Immediate: `(pointer, length, A, B)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// event = String(pointer, length)
    /// new_callback = ProxyToRustCallback(A, B);
    /// stack.top().updateEventlistener(new_callback)
    /// ```
    UpdateEventListener = 12,

    /// Immediates: `(pointer, length)`
    ///
    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().removeEventListener(String(pointer, length));
    /// ```
    RemoveEventListener = 13,
}

// Allocation utilities to ensure that we only allocation sequences of `u32`s
// into the change list's bump arena without any padding. This helps maintain
// the invariants required for `Bump::each_allocated_chunk`'s safety.
impl ChangeList {
    // Allocate an opcode with zero immediates.
    fn op0(&self, discriminant: ChangeDiscriminant) {
        self.bump.alloc(discriminant as u32);
    }

    // Note: no 1-immediate opcodes at this time.

    // Allocate an opcode with two immediates.
    fn op2(&self, discriminant: ChangeDiscriminant, a: u32, b: u32) {
        self.bump.alloc([discriminant as u32, a, b]);
    }

    // Note: no 3-immediate opcodes at this time.

    // Allocate an opcode with four immediates.
    fn op4(&self, discriminant: ChangeDiscriminant, a: u32, b: u32, c: u32, d: u32) {
        self.bump.alloc([discriminant as u32, a, b, c, d]);
    }
}

impl ChangeList {
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

    pub(crate) fn emit_set_attribute(&self, name: &str, value: &str) {
        debug!("emit_set_attribute({:?}, {:?})", name, value);
        self.op4(
            ChangeDiscriminant::SetAttribute,
            name.as_ptr() as u32,
            name.len() as u32,
            value.as_ptr() as u32,
            value.len() as u32,
        );
    }

    pub(crate) fn emit_remove_attribute(&self, name: &str) {
        debug!("emit_remove_attribute({:?})", name);
        self.op2(
            ChangeDiscriminant::RemoveAttribute,
            name.as_ptr() as u32,
            name.len() as u32,
        );
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

    pub(crate) fn emit_create_element(&self, tag_name: &str) {
        debug!("emit_create_element({:?})", tag_name);
        self.op2(
            ChangeDiscriminant::CreateElement,
            tag_name.as_ptr() as u32,
            tag_name.len() as u32,
        );
    }

    pub(crate) fn emit_new_event_listener(&self, listener: &Listener) {
        debug!("emit_new_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        self.op4(
            ChangeDiscriminant::NewEventListener,
            listener.event.as_ptr() as u32,
            listener.event.len() as u32,
            a,
            b,
        );
    }

    pub(crate) fn emit_update_event_listener(&self, listener: &Listener) {
        debug!("emit_update_event_listener({:?})", listener);
        let (a, b) = listener.get_callback_parts();
        debug_assert!(a != 0);
        self.op4(
            ChangeDiscriminant::UpdateEventListener,
            listener.event.as_ptr() as u32,
            listener.event.len() as u32,
            a,
            b,
        );
    }

    pub(crate) fn emit_remove_event_listener(&self, event: &str) {
        debug!("emit_remove_event_listener({:?})", event);
        self.op2(
            ChangeDiscriminant::RemoveEventListener,
            event.as_ptr() as u32,
            event.len() as u32,
        );
    }
}
