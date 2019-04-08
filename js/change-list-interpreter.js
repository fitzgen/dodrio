const decoder = new TextDecoder();

function string(mem, pointer, length) {
  const buf = mem.subarray(pointer, pointer + length);
  return decoder.decode(buf);
}

const OP_TABLE = [
  // 0
  function setText(interpreter, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const str = string(mem8, pointer, length);
    interpreter.top().textContent = str;
    return i;
  },

  // 1
  function removeSelfAndNextSiblings(interpreter, mem8, mem32, i) {
    const node = interpreter.pop();
    let sibling = node.nextSibling;
    while (sibling) {
      const temp = sibling.nextSibling;
      sibling.remove();
      sibling = temp;
    }
    node.remove();
    return i;
  },

  // 2
  function replaceWith(interpreter, mem8, mem32, i) {
    const newNode = interpreter.pop();
    const oldSiblingIndex = interpreter.topSiblingIndex();
    const oldNode = interpreter.pop();
    oldNode.replaceWith(newNode);
    interpreter.push(newNode, oldSiblingIndex);
    return i;
  },

  // 3
  function setAttribute(interpreter, mem8, mem32, i) {
    const nameId = mem32[i++];
    const valueId = mem32[i++];
    const name = interpreter.getCachedString(nameId);
    const value = interpreter.getCachedString(valueId);
    const node = interpreter.top();
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

    return i;
  },

  // 4
  function removeAttribute(interpreter, mem8, mem32, i) {
    const nameId = mem32[i++];
    const name = interpreter.getCachedString(nameId);
    const node = interpreter.top();
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

    return i;
  },

  // 5
  function pushFirstChild(interpreter, mem8, mem32, i) {
    interpreter.push(interpreter.top().firstChild, 0);
    return i;
  },

  // 6
  function popPushNextSibling(interpreter, mem8, mem32, i) {
    const siblingIndex = interpreter.topSiblingIndex();
    const node = interpreter.pop();
    interpreter.push(node.nextSibling, siblingIndex + 1);
    return i;
  },

  // 7
  function pop(interpreter, mem8, mem32, i) {
    interpreter.pop();
    return i;
  },

  // 8
  function appendChild(interpreter, mem8, mem32, i) {
    const child = interpreter.pop();
    interpreter.top().appendChild(child);
    return i;
  },

  // 9
  function createTextNode(interpreter, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const text = string(mem8, pointer, length);
    interpreter.push(document.createTextNode(text), -1);
    return i;
  },

  // 10
  function createElement(interpreter, mem8, mem32, i) {
    const tagNameId = mem32[i++];
    const tagName = interpreter.getCachedString(tagNameId);
    interpreter.push(document.createElement(tagName), -1);
    return i;
  },

  // 11
  function newEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getCachedString(eventId);
    const a = mem32[i++];
    const b = mem32[i++];
    const el = interpreter.top();
    el.addEventListener(eventType, interpreter.eventHandler);
    el[`dodrio-a-${eventType}`] = a;
    el[`dodrio-b-${eventType}`] = b;
    return i;
  },

  // 12
  function updateEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getCachedString(eventId);
    const el = interpreter.top();
    el[`dodrio-a-${eventType}`] = mem32[i++];
    el[`dodrio-b-${eventType}`] = mem32[i++];
    return i;
  },

  // 13
  function removeEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getCachedString(eventId);
    const el = interpreter.top();
    el.removeEventListener(eventType, interpreter.eventHandler);
    return i;
  },

  // 14
  function addCachedString(interpreter, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const id = mem32[i++];
    const str = string(mem8, pointer, length);
    interpreter.addCachedString(str, id);
    return i;
  },

  // 15
  function dropCachedString(interpreter, mem8, mem32, i) {
    const id = mem32[i++];
    interpreter.dropCachedString(id);
    return i;
  },

  // 16
  function createElementNS(interpreter, mem8, mem32, i) {
    const tagNameId = mem32[i++];
    const tagName = interpreter.getCachedString(tagNameId);
    const nsId = mem32[i++];
    const ns = interpreter.getCachedString(nsId);
    interpreter.push(document.createElementNS(ns, tagName), -1);
    return i;
  },

  // 17
  function pushChild(interpreter, mem8, mem32, i) {
    const index = mem32[i++];
    const parent = interpreter.top();
    const child = parent.childNodes[index];
    interpreter.push(child, index);
    return i;
  },

  // 18
  function popPushSibling(interpreter, mem8, mem32, i) {
    const baseIndex = interpreter.topSiblingIndex();
    const node = interpreter.pop();
    const relativeIndex = mem32[i++];
    const absoluteIndex = baseIndex + relativeIndex;
    const sibling = node.parentNode.childNodes[absoluteIndex];
    interpreter.push(sibling, absoluteIndex);
    return i;
  }
];

export class ChangeListInterpreter {
  constructor(container) {
    this.trampoline = null;
    this.container = container;
    this.ranges = [];
    this.stack = [];
    this.siblingIndices = [];
    this.strings = new Map();
  }

  unmount() {
    this.trampoline.mounted = false;

    // Null out all of our properties just to ensure that if we mistakenly ever
    // call a method on this instance again, it will throw.
    this.trampoline = null;
    this.container = null;
    this.ranges = null;
    this.stack = null;
    this.siblingIndices = null;
    this.strings = null;
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
      this.applyChangeRange(mem8, mem32, start, len);
    }

    this.ranges.length = 0;
    this.stack.length = 0;
  }

  applyChangeRange(mem8, mem32, start, len) {
    const end = (start + len) / 4;
    for (let i = start / 4; i < end; ) {
      const op = mem32[i++];
      const opFunc = OP_TABLE[op];
      i = opFunc(this, mem8, mem32, i);
    }
  }

  push(node, siblingIndex) {
    this.stack.push(node);
    this.siblingIndices.push(siblingIndex);
  }

  pop() {
    this.siblingIndices.pop();
    return this.stack.pop();
  }

  top() {
    return this.stack[this.stack.length - 1];
  }

  topSiblingIndex() {
    return this.siblingIndices[this.siblingIndices.length - 1];
  }

  addCachedString(str, id) {
    this.strings.set(id, str);
  }

  dropCachedString(id) {
    this.strings.delete(id);
  }

  getCachedString(id) {
    return this.strings.get(id);
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
}
