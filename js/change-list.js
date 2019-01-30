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
  function setText(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("setText");
    const pointer = mem32[i++];
    const length = mem32[i++];
    const str = string(mem8, pointer, length);
    // console.log("  str =", str);
    // console.log("  top(stack) =", top(stack));
    top(stack).textContent = str;
    return i;
  },

  // 1
  function removeSelfAndNextSiblings(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("removeSelfAndNextSiblings");
    const node = stack.pop();
    // console.log("  top(stack) =", node);
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
  function replaceWith(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("replaceWith");
    const newNode = stack.pop();
    // console.log("  newNode =", newNode);
    const oldNode = stack.pop();
    // console.log("  oldNode =", oldNode);
    oldNode.replaceWith(newNode);
    stack.push(newNode);
    return i;
  },

  // 3
  function setAttribute(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("setAttribute");
    const pointer1 = mem32[i++];
    const length1 = mem32[i++];
    const name = string(mem8, pointer1, length1);
    // console.log("  name =", name);
    const pointer2 = mem32[i++];
    const length2 = mem32[i++];
    const value = string(mem8, pointer2, length2);
    // console.log("  value =", value);
    // console.log("  top(stack) =", top(stack));
    top(stack).setAttribute(name, value);
    return i;
  },

  // 4
  function removeAttribute(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("removeAttribute");
    const pointer = mem32[i++];
    const length = mem32[i++];
    const name = string(mem8, pointer, length);
    // console.log("  name =", name);
    top(stack).removeAttribute(name);
    return i;
  },

  // 5
  function pushFirstChild(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("pushFirstChild");
    // console.log("  top(stack) =", top(stack));
    stack.push(top(stack).firstChild);
    return i;
  },

  // 6
  function popPushNextSibling(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("popPushNextSibling");
    // console.log("  top(stack) =", top(stack));
    const node = stack.pop();
    stack.push(node.nextSibling);
    return i;
  },

  // 7
  function pop(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("pop");
    // console.log("  top(stack) =", top(stack));
    stack.pop();
    return i;
  },

  // 8
  function appendChild(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("appendChild");
    // console.log("  top(stack) =", top(stack));
    const child = stack.pop();
    top(stack).appendChild(child);
    return i;
  },

  // 9
  function createTextNode(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("appendChild");
    const pointer = mem32[i++];
    const length = mem32[i++];
    const text = string(mem8, pointer, length);
    // console.log("  text =", text);
    // console.log("  top(stack) =", top(stack));
    stack.push(document.createTextNode(text));
    return i;
  },

  // 10
  function createElement(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    // console.log("createElement");
    const pointer = mem32[i++];
    const length = mem32[i++];
    const tagName = string(mem8, pointer, length);
    // console.log("  tagName =", tagName);
    // console.log("  top(stack) =", top(stack));
    stack.push(document.createElement(tagName));
    return i;
  },

  // 11
  function newEventListener(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const eventType = string(mem8, pointer, length);
    const a = mem32[i++];
    const b = mem32[i++];
    const el = top(stack);
    const listener = new Listener(a, b, eventType, eventsTrampoline, el);
    listeners.add(listener);
    el.addEventListener(eventType, listener.callback);
    el[`dodrio-${eventType}`] = listener;
    return i;
  },

  // 12
  function updateEventListener(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const eventType = string(mem8, pointer, length);
    const el = top(stack);
    const listener = el[`dodrio-${eventType}`];
    listener.a = mem32[i++];
    listener.b = mem32[i++];
    return i;
  },

  // 13
  function removeEventListener(stack, eventsTrampoline, listeners, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const eventType = string(mem8, pointer, length);
    const el = top(stack);
    const listener = el[`dodrio-${eventType}`];
    el.removeEventListener(eventType, listener.callback);
    listeners.delete(listener);
    return i;
  }
];

class Listener {
  constructor(a, b, eventType, trampoline, el) {
    this.a = a;
    this.b = b;
    this.eventType = eventType;
    this.trampoline = trampoline;
    this.el = el;
    this.callback = this.callback.bind(this);
  }

  callback(event) {
    this.trampoline(event, this.a, this.b);
  }
}

// export
class ChangeList {
  constructor(container) {
    this.listeners = new Set();
    this.trampoline = null;
    this.container = container;
    this.ranges = [];
    this.stack = [];
  }

  unmount() {
    for (const listener of this.listeners) {
      listener.el.removeEventListener(listener.eventType, listener.callback);
      listener.trampoline = () => {
        throw new Error("invocation of listener after it has been removed");
      };
      listener.a = 0;
      listener.b = 0;
    }

    // Null out all of our properties just to ensure that if we mistakenly ever
    // call a method on this instance again, it will throw.
    this.listeners = null;
    this.trampoline = null;
    this.container = null;
    this.ranges = null;
    this.stack = null;
  }

  addChangeListRange(start, len) {
    this.ranges.push(start);
    this.ranges.push(len);
  }

  applyChanges(memory) {
    // console.log("======== ChangeList.prototype.applyChanges ==============================");
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
      // console.log();
      // console.log(OP_TABLE[op].name);
      // console.log(this.stack);
      i = OP_TABLE[op](this.stack, this.eventsTrampoline, this.listeners, mem8, mem32, i);
    }
  }

  initEventsTrampoline(trampoline) {
    this.eventsTrampoline = (...args) => {
      trampoline(...args);
    };
  }
}
window.ChangeList = ChangeList;
