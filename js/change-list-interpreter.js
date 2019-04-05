const decoder = new TextDecoder();

function top(stack) {
  return stack[stack.length - 1];
}

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
    top(interpreter.stack).textContent = str;
    return i;
  },

  // 1
  function removeSelfAndNextSiblings(interpreter, mem8, mem32, i) {
    const node = interpreter.stack.pop();
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
    const newNode = interpreter.stack.pop();
    const oldNode = interpreter.stack.pop();
    oldNode.replaceWith(newNode);
    interpreter.stack.push(newNode);
    return i;
  },

  // 3
  function setAttribute(interpreter, mem8, mem32, i) {
    const nameId = mem32[i++];
    const valueId = mem32[i++];
    const name = interpreter.getString(nameId);
    const value = interpreter.getString(valueId);
    const node = top(interpreter.stack);
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
    const name = interpreter.getString(nameId);
    const node = top(interpreter.stack);
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
    interpreter.stack.push(top(interpreter.stack).firstChild);
    return i;
  },

  // 6
  function popPushNextSibling(interpreter, mem8, mem32, i) {
    const node = interpreter.stack.pop();
    interpreter.stack.push(node.nextSibling);
    return i;
  },

  // 7
  function pop(interpreter, mem8, mem32, i) {
    interpreter.stack.pop();
    return i;
  },

  // 8
  function appendChild(interpreter, mem8, mem32, i) {
    const child = interpreter.stack.pop();
    top(interpreter.stack).appendChild(child);
    return i;
  },

  // 9
  function createTextNode(interpreter, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const text = string(mem8, pointer, length);
    interpreter.stack.push(document.createTextNode(text));
    return i;
  },

  // 10
  function createElement(interpreter, mem8, mem32, i) {
    const tagNameId = mem32[i++];
    const tagName = interpreter.getString(tagNameId);
    interpreter.stack.push(document.createElement(tagName));
    return i;
  },

  // 11
  function newEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getString(eventId);
    const a = mem32[i++];
    const b = mem32[i++];
    const el = top(interpreter.stack);
    el.addEventListener(eventType, interpreter.eventHandler);
    el[`dodrio-a-${eventType}`] = a;
    el[`dodrio-b-${eventType}`] = b;
    return i;
  },

  // 12
  function updateEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getString(eventId);
    const el = top(interpreter.stack);
    el[`dodrio-a-${eventType}`] = mem32[i++];
    el[`dodrio-b-${eventType}`] = mem32[i++];
    return i;
  },

  // 13
  function removeEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getString(eventId);
    const el = top(interpreter.stack);
    el.removeEventListener(eventType, interpreter.eventHandler);
    return i;
  },

  // 14
  function addString(interpreter, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const id = mem32[i++];
    const str = string(mem8, pointer, length);
    interpreter.addString(str, id);
    return i;
  },

  // 15
  function dropString(interpreter, mem8, mem32, i) {
    const id = mem32[i++];
    interpreter.dropString(id);
    return i;
  },

  // 16
  function createElementNS(interpreter, mem8, mem32, i) {
    const tagNameId = mem32[i++];
    const tagName = interpreter.getString(tagNameId);
    const nsId = mem32[i++];
    const ns = interpreter.getString(nsId);
    interpreter.stack.push(document.createElementNS(ns, tagName));
    return i;
  },

  // 17
  function setAttributeNS(interpreter, mem8, mem32, i) {
    const nameId = mem32[i++];
    const valueId = mem32[i++];
    const name = interpreter.getString(nameId);
    const value = interpreter.getString(valueId);
    const node = top(interpreter.stack);
    node.setAttributeNS(null, name, value);
    return i;
  }
];

export class ChangeListInterpreter {
  constructor(container) {
    this.trampoline = null;
    this.container = container;
    this.ranges = [];
    this.stack = [];
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
      i = OP_TABLE[op](this, mem8, mem32, i);
    }
  }

  addString(str, id) {
    this.strings.set(id, str);
  }

  dropString(id) {
    this.strings.delete(id);
  }

  getString(id) {
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
