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
    const name = interpreter.getCachedString(nameId);
    const value = interpreter.getCachedString(valueId);
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
    const name = interpreter.getCachedString(nameId);
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
  function pushReverseChild(interpreter, mem8, mem32, i) {
    const n = mem32[i++];
    const parent = top(interpreter.stack);
    const children = parent.childNodes;
    const child = children[children.length - n - 1];
    interpreter.stack.push(child);
    return i;
  },

  // 6
  function popPushChild(interpreter, mem8, mem32, i) {
    const n = mem32[i++];
    interpreter.stack.pop();
    const parent = top(interpreter.stack);
    const children = parent.childNodes;
    const child = children[n];
    interpreter.stack.push(child);
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
    const tagName = interpreter.getCachedString(tagNameId);
    interpreter.stack.push(document.createElement(tagName));
    return i;
  },

  // 11
  function newEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getCachedString(eventId);
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
    const eventType = interpreter.getCachedString(eventId);
    const el = top(interpreter.stack);
    el[`dodrio-a-${eventType}`] = mem32[i++];
    el[`dodrio-b-${eventType}`] = mem32[i++];
    return i;
  },

  // 13
  function removeEventListener(interpreter, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = interpreter.getCachedString(eventId);
    const el = top(interpreter.stack);
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
    interpreter.stack.push(document.createElementNS(ns, tagName));
    return i;
  },

  // 17
  function saveChildrenToTemporaries(interpreter, mem8, mem32, i) {
    let temp = mem32[i++];
    const start = mem32[i++];
    const end = mem32[i++];
    const parent = top(interpreter.stack);
    const children = parent.childNodes;
    for (let i = start; i < end; i++) {
      interpreter.temporaries[temp++] = children[i];
    }
    return i;
  },

  // 18
  function pushChild(interpreter, mem8, mem32, i) {
    const parent = top(interpreter.stack);
    const n = mem32[i++];
    const child = parent.childNodes[n];
    interpreter.stack.push(child);
    return i;
  },

  // 19
  function pushTemporary(interpreter, mem8, mem32, i) {
    const temp = mem32[i++];
    interpreter.stack.push(interpreter.temporaries[temp]);
    return i;
  },

  // 20
  function insertBefore(interpreter, mem8, mem32, i) {
    const before = interpreter.stack.pop();
    const after = interpreter.stack.pop();
    after.parentNode.insertBefore(before, after);
    interpreter.stack.push(before);
    return i;
  },

  // 21
  function popPushReverseChild(interpreter, mem8, mem32, i) {
    const n = mem32[i++];
    interpreter.stack.pop();
    const parent = top(interpreter.stack);
    const children = parent.childNodes;
    const child = children[children.length - n - 1];
    interpreter.stack.push(child);
    return i;
  },

  // 22
  function removeChild(interpreter, mem8, mem32, i) {
    const n = mem32[i++];
    const parent = top(interpreter.stack);
    const child = parent.childNodes[n];
    child.remove();
    return i;
  },

  // 23
  function setClass(interpreter, mem8, mem32, i) {
    const classId = mem32[i++];
    const className = interpreter.getCachedString(classId);
    top(interpreter.stack).className = className;
    return i;
  },

  // 24
  function saveTemplate(interpreter, mem8, mem32, i) {
    const id = mem32[i++];
    const template = top(interpreter.stack);
    interpreter.saveTemplate(id, template.cloneNode(true));
    return i;
  },

  // 25
  function pushTemplate(interpreter, mem8, mem32, i) {
    const id = mem32[i++];
    const template = interpreter.getTemplate(id);
    interpreter.stack.push(template.cloneNode(true));
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
      this.applyChangeRange(mem8, mem32, start, len);
    }

    this.ranges.length = 0;
    this.stack.length = 0;
    this.temporaries.length = 0;
  }

  applyChangeRange(mem8, mem32, start, len) {
    const end = (start + len) / 4;
    for (let i = start / 4; i < end; ) {
      const op = mem32[i++];
      i = OP_TABLE[op](this, mem8, mem32, i);
    }
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

  saveTemplate(id, template) {
    this.templates.set(id, template);
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
}
