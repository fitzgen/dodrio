const decoder = new TextDecoder();

function top(stack) {
  return stack[stack.length - 1];
}

function string(mem, pointer, length) {
  const buf = mem.subarray(pointer, pointer + length);
  return decoder.decode(buf);
}

export class ChangeListInterpreter {
  constructor(container) {
    this.trampoline = null;
    this.container = container;
    this.ranges = [];
    this.stack = [];
    this.strings = new Map();
    this.temporaries = [];
    this.templates = new Map();
  }

  unmount() {
    this.trampoline.mounted = false;

    // Null out all of our properties just to ensure that if we mistakenly ever
    // call a method on this instance again, it will throw.
    this.trampoline = null;
    this.container = null;
    this.ranges = null;
    this.stack = null;
    this.strings = null;
    this.temporaries = null;
    this.templates = null;
  }

  addChangeListRange(start, len) {
    this.ranges.push(start);
    this.ranges.push(len);
  }

  applyChanges(memory) {
    if (this.ranges.length == 0) {
      return;
    }

    this.stack.push(this.container.firstChild);
    const mem8 = new Uint8Array(memory.buffer);
    const mem32 = new Uint32Array(memory.buffer);

    for (let i = 0; i < this.ranges.length; i += 2) {
      const start = this.ranges[i];
      const len = this.ranges[i + 1];
      this.applyChangeRange(mem8, mem32, start, len, memory);
    }

    this.reset();
  }

  start() {
    this.stack.push(this.container.firstChild);
  }

  reset() {
    this.ranges.length = 0;
    this.stack.length = 0;
    this.temporaries.length = 0;
  }

  applyChangeRange(mem8, mem32, start, len, memory) {
    const end = (start + len) / 4;
    for (let i = start / 4; i < end; ) {
      const op = mem32[i++];
      i = OP_TABLE[op](this, mem8, mem32, i, memory);
    }
  }

  getCachedString(id) {
    return this.strings.get(id);
  }

  getTemplate(id) {
    return this.templates.get(id);
  }

  initEventsTrampoline(trampoline) {
    this.trampoline = trampoline;
    trampoline.mounted = true;
    this.eventHandler = function(event) {
      if (!trampoline.mounted) {
        throw new Error("invocation of listener after VDOM has been unmounted");
      }

      // `this` always refers to the element the handler was added to.
      // Since we're adding the handler to all elements our content wants
      // to listen for events on, this ensures that we always get the right
      // values for `a` and `b`.
      const type = event.type;
      const a = this[`dodrio-a-${type}`];
      const b = this[`dodrio-b-${type}`];
      trampoline(event, a, b);
    }
  }

  // 0
  setText(pointer, length, memory) {
    const mem8 = new Uint8Array(memory.buffer);  
    const str = string(mem8, pointer, length);
    top(this.stack).textContent = str;
  }

  // 1
  removeSelfAndNextSiblings(interpreter) {
    const node = this.stack.pop();
    let sibling = node.nextSibling;
    while (sibling) {
      const temp = sibling.nextSibling;
      sibling.remove();
      sibling = temp;
    }
    node.remove();
  }

  // 2
  replaceWith(interpreter) {
    const newNode = this.stack.pop();
    const oldNode = this.stack.pop();
    oldNode.replaceWith(newNode);
    this.stack.push(newNode);
  }

  // 3
  setAttribute(nameId, valueId) {
    const name = this.getCachedString(nameId);
    const value = this.getCachedString(valueId);
    const node = top(this.stack);
    node.setAttribute(name, value);

    // Some attributes are "volatile" and don't work through `setAttribute`.
    if (name === "value") {
      node.value = value;
    }
    if (name === "checked") {
      node.checked = true;
    }
    if (name === "selected") {
      node.selected = true;
    }
  }

  // 4
  removeAttribute(nameId) {
    const name = this.getCachedString(nameId);
    const node = top(this.stack);
    node.removeAttribute(name);

    // Some attributes are "volatile" and don't work through `removeAttribute`.
    if (name === "value") {
      node.value = null;
    }
    if (name === "checked") {
      node.checked = false;
    }
    if (name === "selected") {
      node.selected = false;
    }
  }

  // 5
  pushReverseChild(n) {
    const parent = top(this.stack);
    const children = parent.childNodes;
    const child = children[children.length - n - 1];
    this.stack.push(child);
  }

  // 6
  popPushChild(n) {
    this.stack.pop();
    const parent = top(this.stack);
    const children = parent.childNodes;
    const child = children[n];
    this.stack.push(child);
  }

  // 7
  pop(interpreter) {
    this.stack.pop();
  }

  // 8
  appendChild(interpreter) {
    const child = this.stack.pop();
    top(this.stack).appendChild(child);
  }

  // 9
  createTextNode(pointer, length, memory) {
    const mem8 = new Uint8Array(memory.buffer);
    const text = string(mem8, pointer, length);
    this.stack.push(document.createTextNode(text));
  }

  // 10
  createElement(tagNameId) {
    const tagName = this.getCachedString(tagNameId);
    this.stack.push(document.createElement(tagName));
  }

  // 11
  newEventListener(eventId, a, b) {
    const eventType = this.getCachedString(eventId);
    const el = top(this.stack);
    el.addEventListener(eventType, this.eventHandler);
    el[`dodrio-a-${eventType}`] = a;
    el[`dodrio-b-${eventType}`] = b;
  }

  // 12
  updateEventListener(eventId, a, b) {
    const eventType = this.getCachedString(eventId);
    const el = top(this.stack);
    el[`dodrio-a-${eventType}`] = a;
    el[`dodrio-b-${eventType}`] = b;
  }

  // 13
  removeEventListener(eventId) {
    const eventType = this.getCachedString(eventId);
    const el = top(this.stack);
    el.removeEventListener(eventType, this.eventHandler);
  }

  // 14
  addCachedString(pointer, length, id, memory) {
    const mem8 = new Uint8Array(memory.buffer);
    const str = string(mem8, pointer, length);
    this.strings.set(id, str);
  }

  // 15
  dropCachedString(id) {
    this.strings.delete(id);
  }

  // 16
  createElementNS(tagNameId, nsId) {
    const tagName = this.getCachedString(tagNameId);
    const ns = this.getCachedString(nsId);
    this.stack.push(document.createElementNS(ns, tagName));
  }

  // 17
  saveChildrenToTemporaries(temp, start, end) {
    const parent = top(this.stack);
    const children = parent.childNodes;
    for (let i = start; i < end; i++) {
      this.temporaries[temp++] = children[i];
    }
  }

  // 18
  pushChild(n) {
    const parent = top(this.stack);
    const child = parent.childNodes[n];
    this.stack.push(child);
  }

  // 19
  pushTemporary(temp) {
    this.stack.push(this.temporaries[temp]);
  }

  // 20
  insertBefore(interpreter) {
    const before = this.stack.pop();
    const after = this.stack.pop();
    after.parentNode.insertBefore(before, after);
    this.stack.push(before);
  }

  // 21
  popPushReverseChild(n) {
    this.stack.pop();
    const parent = top(this.stack);
    const children = parent.childNodes;
    const child = children[children.length - n - 1];
    this.stack.push(child);
  }

  // 22
  removeChild(n) {
    const parent = top(this.stack);
    const child = parent.childNodes[n];
    child.remove();
  }

  // 23
  setClass(classId) {
    const className = this.getCachedString(classId);
    top(this.stack).className = className;
  }

  // 24
  saveTemplate(id) {
    const template = top(this.stack);
    this.templates.set(id, template.cloneNode(true));
  }

  // 25
  pushTemplate(id) {
    const template = this.getTemplate(id);
    this.stack.push(template.cloneNode(true));
  }
}
