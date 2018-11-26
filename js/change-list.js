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
  function setText(stack, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const str = string(mem8, pointer, length);
    top(stack).textContent = str;
    return i;
  },

  // 1
  function removeSelfAndNextSiblings(stack, mem8, mem32, i) {
    const node = stack.pop();
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
  function replaceWith(stack, mem8, mem32, i) {
    const newNode = stack.pop();
    const oldNode = stack.pop();
    oldNode.replaceWith(newNode);
    stack.push(newNode);
    return i;
  },

  // 3
  function setAttribute(stack, mem8, mem32, i) {
    const pointer1 = mem32[i++];
    const length1 = mem32[i++];
    const name = string(mem8, pointer1, length1);
    const pointer2 = mem32[i++];
    const length2 = mem32[i++];
    const value = string(mem8, pointer2, length2);
    top(stack).setAttribute(name, value);
    return i;
  },

  // 4
  function removeAttribute(stack, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const name = string(mem8, pointer, length);
    top(stack).removeAttribute(name);
    return i;
  },

  // 5
  function pushFirstChild(stack, mem8, mem32, i) {
    stack.push(top(stack).firstChild);
    return i;
  },

  // 6
  function popPushNextSibling(stack, mem8, mem32, i) {
    const node = stack.pop();
    stack.push(node.nextSibling);
    return i;
  },

  // 7
  function pop(stack, mem8, mem32, i) {
    stack.pop();
    return i;
  },

  // 8
  function appendChild(stack, mem8, mem32, i) {
    const child = stack.pop();
    top(stack).appendChild(child);
    return i;
  },

  // 9
  function createTextNode(stack, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const text = string(mem8, pointer, length);
    stack.push(document.createTextNode(text));
    return i;
  },

  // 10
  function createElement(stack, mem8, mem32, i) {
    const pointer = mem32[i++];
    const length = mem32[i++];
    const tagName = string(mem8, pointer, length);
    stack.push(document.createElement(tagName));
    return i;
  }
];

export class ChangeList {
  constructor(container) {
    this.container = container;
    this.ranges = [];
    this.stack = [];
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
      i = OP_TABLE[op](this.stack, mem8, mem32, i);
    }
  }
}
