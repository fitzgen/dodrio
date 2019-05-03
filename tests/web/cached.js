let cloneNodeCount = 0;

const origCloneNode = Node.prototype.cloneNode;
Node.prototype.cloneNode = function (...args) {
  cloneNodeCount += 1;
  return origCloneNode.apply(this, args);
};

export function getCloneNodeCount() {
  return cloneNodeCount;
}
