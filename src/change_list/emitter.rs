//! The `InstructionEmitter` is responsible for encoding change list
//! instructions and ensuring that each instruction has the correct number of
//! immediates. It ensures that the resulting change list instruction stream is
//! *syntactically* correct (opcodes have the correct arity of immediate
//! arguments, etc), while `ChangeListBuilder` wraps an emitter and additionally
//! ensures that the resulting change list program is *semantically* correct
//! (doesn't reference cached strings before they've been added to the cache,
//! etc).
//!
//! We encode the instructions directly into a dedicated bump arena. We
//! eventually pass the bump arena's chunks to the interpreter in JS, so it is
//! critical that nothing other than change list instructions are allocated
//! inside this bump, and that the instructions themselves do not contain any
//! padding or uninitialized memory. See the documentation for the the
//! `Bump::each_allocated_chunk` method for details.

use bumpalo::Bump;

#[derive(Debug)]
pub(crate) struct InstructionEmitter {
    bump: Bump,
}

impl InstructionEmitter {
    /// Construct a new `InstructionEmitter` with its own bump arena.
    pub fn new() -> InstructionEmitter {
        let bump = Bump::new();
        InstructionEmitter { bump }
    }

    /// Invoke the given function with each of the allocated instruction
    /// sequences that this emitter has built up.
    #[cfg_attr(feature = "xxx-unstable-internal-use-only", allow(dead_code))]
    pub fn each_instruction_sequence<F>(&mut self, f: F)
    where
        F: FnMut(&[u8]),
    {
        // Note: the safety invariants required for `each_allocated_chunk` are
        // maintained by the fact that everything we encode as allocations in
        // the bump arena are in the form of `u32`s, and therefore we don't have
        // any uninitialized memory padding in the arena.
        unsafe {
            self.bump.each_allocated_chunk(f);
        }
    }

    /// Reset to an empty sequence of instructions.
    pub fn reset(&mut self) {
        self.bump.reset();
    }
}

macro_rules! define_change_list_instructions {
    ( $(
        $( #[$attr:meta] )*
        $name:ident (
            $($immediate:ident),*
        ) = $discriminant:expr,
    )* ) => {
        impl InstructionEmitter {
            $(
                $( #[$attr] )*
                #[inline]
                pub fn $name(&self $(, $immediate: u32)*) {
                    self.bump.alloc_with(|| [$discriminant $(, $immediate )* ]);
                }
            )*
        }
    }
}

define_change_list_instructions! {
    /// Stack: `[... TextNode] -> [... TextNode]`
    ///
    /// ```text
    /// stack.top().textContent = readString(pointer, length)
    /// ```
    set_text(pointer, length) = 0,

    /// Stack: `[... Node] -> [...]`
    ///
    /// ```text
    /// node = stack.pop()
    /// while (node.nextSibling) {
    ///   node.nextSibling.remove();
    /// }
    /// node.remove()
    /// ```
    remove_self_and_next_siblings() = 1,

    /// Stack: `[... Node Node] -> [... Node]`
    ///
    /// ```text
    /// new = stack.pop()
    /// old = stack.pop()
    /// old.replaceWith(new)
    /// stack.push(new)
    /// ```
    replace_with() = 2,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().setAttribute(getCachedString(attribute_key), getCachedString(value_key))
    /// ```
    set_attribute(attribute_key, value_key) = 3,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().removeAttribute(getCachedString(attribute_key))
    /// ```
    remove_attribute(attribute_key) = 4,

    /// Stack: `[... Node] -> [... Node Node]`
    ///
    /// ```text
    /// parent = stack.top()
    /// child = parent.childNodes[parent.childNodes.length - n - 1]
    /// stack.push(child)
    /// ```
    push_reverse_child(n) = 5,

    /// Stack: `[... Node Node] -> [... Node Node]`
    ///
    /// ```text
    /// stack.pop();
    /// parent = stack.top();
    /// child = parent.childNodes[n]
    /// stack.push(child)
    /// ```
    pop_push_child(n) = 6,

    /// Stack: `[... T] -> [...]`
    ///
    /// ```text
    /// stack.pop()
    /// ```
    pop() = 7,

    /// Stack: `[... Node Node] -> [... Node]`
    ///
    /// ```text
    /// child = stack.pop()
    /// stack.top().appendChild(child)
    /// ```
    append_child() = 8,

    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(document.createTextNode(readString(pointer, length)))
    /// ```
    create_text_node(pointer, length) = 9,

    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(document.createElement(getCachedString(tag_name_key))
    /// ```
    create_element(tag_name_key) = 10,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// event = getCachedString(event_key)
    /// callback = createProxyToRustCallback(a, b)
    /// stack.top().addEventListener(event, callback)
    /// ```
    new_event_listener(event_key, a, b) = 11,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// event = getCachedString(event_key)
    /// new_callback = createProxyToRustCallback(a, b);
    /// stack.top().updateEventlistener(new_callback)
    /// ```
    update_event_listener(event_key, a, b) = 12,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// stack.top().removeEventListener(getCachedString(event_key));
    /// ```
    remove_event_listener(event_key) = 13,

    /// Stack: `[...] -> [...]`
    ///
    /// ```text
    /// addCachedString(readString(pointer, length), key);
    /// ```
    add_cached_string(pointer, length, key) = 14,

    /// Stack: `[...] -> [...]`
    ///
    /// ```text
    /// dropCachedString(key);
    /// ```
    drop_cached_string(key) = 15,

    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// tag_name = getCachedString(tag_name_key)
    /// namespace = getCachedString(tag_name_key)
    /// stack.push(document.createElementNS(tag_name, namespace))
    /// ```
    create_element_ns(tag_name_key, namespace_key) = 16,

    /// Stack: `[...] -> [...]`
    ///
    /// ```text
    /// parent = stack.top()
    /// children = parent.childNodes
    /// temp = temp_base
    /// for i in start .. end:
    ///     temporaries[temp] = children[i]
    ///     temp += 1
    /// ```
    save_children_to_temporaries(temp_base, start, end) = 17,

    /// Stack: `[... Node] -> [... Node Node]`
    ///
    /// ```text
    /// parent = stack.top()
    /// child = parent.childNodes[n]
    /// stack.push(child)
    /// ```
    push_child(n) = 18,

    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// stack.push(temporaries[temp])
    /// ```
    push_temporary(temp) = 19,

    /// Stack: `[... Node Node] -> [... Node]`
    ///
    /// ```text
    /// before = stack.pop()
    /// after = stack.pop()
    /// after.insertBefore(before)
    /// stack.push(before)
    /// ```
    insert_before() = 20,

    /// Stack: `[... Node Node] -> [... Node Node]`
    ///
    /// ```text
    /// stack.pop()
    /// parent = stack.top()
    /// child = parent.childNodes[parent.childNodes.length - n - 1]
    /// stack.push(child)
    /// ```
    pop_push_reverse_child(n) = 21,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// parent = stack.top()
    /// child = parent.childNodes[n]
    /// child.remove()
    /// ```
    remove_child(n) = 22,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// class = getCachedString(class)
    /// node = stack.top()
    /// node.className = class
    /// ```
    set_class(class) = 23,

    /// Stack: `[... Node] -> [... Node]`
    ///
    /// ```text
    /// template = stack.top()
    /// saveTemplate(id, template)
    /// ```
    save_template(id) = 24,

    /// Stack: `[...] -> [... Node]`
    ///
    /// ```text
    /// template = getTemplate(id)
    /// stack.push(template.cloneNode(true))
    /// ```
    push_template(id) = 25,
}
