use bumpalo::{Bump, BumpAllocSafe};
use wasm_bindgen::prelude::*;

pub mod js {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(module = "dodrio/change-list")]
    extern "C" {
        #[derive(Clone, Debug)]
        pub type ChangeList;

        #[wasm_bindgen(constructor)]
        pub fn new(container: &web_sys::Node) -> ChangeList;

        #[wasm_bindgen(method, js_name = addChangeListRange)]
        pub fn add_change_list_range(this: &ChangeList, start: usize, len: usize);

        #[wasm_bindgen(method, js_name = applyChanges)]
        pub fn apply_changes(this: &ChangeList, memory: JsValue);
    }
}

pub(crate) struct ChangeList {
    bump: Bump,
    js: js::ChangeList,
}

impl ChangeList {
    pub(crate) fn new(container: &web_sys::Node) -> ChangeList {
        let bump = Bump::new();
        let js = js::ChangeList::new(container);
        ChangeList { bump, js }
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
}

#[wasm_bindgen]
#[repr(u32)]
pub enum ChangeDiscriminant {
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
}

impl BumpAllocSafe for ChangeDiscriminant {}

impl ChangeList {
    pub(crate) fn emit_set_text(&self, text: &str) {
        debug!("emit_set_text({:?})", text);
        self.bump
            .alloc((ChangeDiscriminant::SetText, text.as_ptr(), text.len()));
    }

    pub(crate) fn emit_remove_self_and_next_siblings(&self) {
        debug!("emit_remove_self_and_next_siblings()");
        self.bump
            .alloc(ChangeDiscriminant::RemoveSelfAndNextSiblings);
    }

    pub(crate) fn emit_replace_with(&self) {
        debug!("emit_replace_with()");
        self.bump.alloc(ChangeDiscriminant::ReplaceWith);
    }

    pub(crate) fn emit_set_attribute(&self, name: &str, value: &str) {
        debug!("emit_set_attribute({:?}, {:?})", name, value);
        self.bump.alloc((
            ChangeDiscriminant::SetAttribute,
            name.as_ptr(),
            name.len(),
            value.as_ptr(),
            value.len(),
        ));
    }

    pub(crate) fn emit_remove_attribute(&self, name: &str) {
        debug!("emit_remove_attribute({:?})", name);
        self.bump.alloc((
            ChangeDiscriminant::RemoveAttribute,
            name.as_ptr(),
            name.len(),
        ));
    }

    pub(crate) fn emit_push_first_child(&self) {
        debug!("emit_push_first_child()");
        self.bump.alloc(ChangeDiscriminant::PushFirstChild);
    }

    pub(crate) fn emit_pop_push_next_sibling(&self) {
        debug!("emit_pop_push_next_sibling()");
        self.bump.alloc(ChangeDiscriminant::PopPushNextSibling);
    }

    pub(crate) fn emit_pop(&self) {
        debug!("emit_pop()");
        self.bump.alloc(ChangeDiscriminant::Pop);
    }

    pub(crate) fn emit_append_child(&self) {
        debug!("emit_append_child()");
        self.bump.alloc(ChangeDiscriminant::AppendChild);
    }

    pub(crate) fn emit_create_text_node(&self, text: &str) {
        debug!("emit_create_text_node({:?})", text);
        self.bump.alloc((
            ChangeDiscriminant::CreateTextNode,
            text.as_ptr(),
            text.len(),
        ));
    }

    pub(crate) fn emit_create_element(&self, tag_name: &str) {
        debug!("emit_create_element({:?})", tag_name);
        self.bump.alloc((
            ChangeDiscriminant::CreateElement,
            tag_name.as_ptr(),
            tag_name.len(),
        ));
    }
}
