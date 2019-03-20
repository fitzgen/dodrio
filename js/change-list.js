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
  function setText(changeList, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const str = string(mem8, pointer, length);
    top(changeList.stack).textContent = str;
    return i;
  },

  // 1
  function removeSelfAndNextSiblings(changeList, mem8, mem32, i) {
    const node = changeList.stack.pop();
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
  function replaceWith(changeList, mem8, mem32, i) {
    const newNode = changeList.stack.pop();
    const oldNode = changeList.stack.pop();
    oldNode.replaceWith(newNode);
    changeList.stack.push(newNode);
    return i;
  },

  // 3
  function setAttribute(changeList, mem8, mem32, i) {
    const nameId = mem32[i++];
    const valueId = mem32[i++];
    const name = changeList.getString(nameId);
    const value = changeList.getString(valueId);
    const node = top(changeList.stack);
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
  function removeAttribute(changeList, mem8, mem32, i) {
    const nameId = mem32[i++];
    const name = changeList.getString(nameId);
    const node = top(changeList.stack);
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
  function pushFirstChild(changeList, mem8, mem32, i) {
    changeList.stack.push(top(changeList.stack).firstChild);
    return i;
  },

  // 6
  function popPushNextSibling(changeList, mem8, mem32, i) {
    const node = changeList.stack.pop();
    changeList.stack.push(node.nextSibling);
    return i;
  },

  // 7
  function pop(changeList, mem8, mem32, i) {
    changeList.stack.pop();
    return i;
  },

  // 8
  function appendChild(changeList, mem8, mem32, i) {
    const child = changeList.stack.pop();
    top(changeList.stack).appendChild(child);
    return i;
  },

  // 9
  function createTextNode(changeList, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const text = string(mem8, pointer, length);
    changeList.stack.push(document.createTextNode(text));
    return i;
  },

  // 10
  function createElement(changeList, mem8, mem32, i) {
    const tagNameId = mem32[i++];
    const tagName = changeList.getString(tagNameId);
    changeList.stack.push(document.createElement(tagName));
    return i;
  },

  // 11
  function newEventListener(changeList, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = changeList.getString(eventId);
    const a = mem32[i++];
    const b = mem32[i++];
    const el = top(changeList.stack);
    el.addEventListener(eventType, changeList.eventHandler);
    el[`dodrio-a-${eventType}`] = a;
    el[`dodrio-b-${eventType}`] = b;
    return i;
  },

  // 12
  function updateEventListener(changeList, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = changeList.getString(eventId);
    const el = top(changeList.stack);
    el[`dodrio-a-${eventType}`] = mem32[i++];
    el[`dodrio-b-${eventType}`] = mem32[i++];
    return i;
  },

  // 13
  function removeEventListener(changeList, mem8, mem32, i) {
    const eventId = mem32[i++];
    const eventType = changeList.getString(eventId);
    const el = top(changeList.stack);
    el.removeEventListener(eventType, changeList.eventHandler);
    return i;
  },

  // 14
  function addString(changeList, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const id = mem32[i++];
    const str = string(mem8, pointer, length);
    changeList.addString(str, id);
    return i;
  },

  // 15
  function dropString(changeList, mem8, mem32, i) {
    const id = mem32[i++];
    changeList.dropString(id);
    return i;
  },

  // 16
  function createElementNS(changeList, mem8, mem32, i) {
    const tagNameId = mem32[i++];
    const tagName = changeList.getString(tagNameId);
    const nsId = mem32[i++];
    const ns = changeList.getString(nsId);
    changeList.stack.push(document.createElementNS(ns, tagName));
    return i;
  },

  // 17
  function setAttributeNS(changeList, mem8, mem32, i) {
    const nameId = mem32[i++];
    const valueId = mem32[i++];
    const name = changeList.getString(nameId);
    const value = changeList.getString(valueId);
    const node = top(changeList.stack);
    node.setAttributeNS(null, name, value);
    return i;
  }
];

// export
class ChangeList {
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
window.ChangeList = ChangeList;

//# sourceURL=dodrio/change-list.js
