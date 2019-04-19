export function saveKeyedElements(container) {
  const saved = new Map;
  const children = container.querySelectorAll(".keyed");
  for (const child of children) {
    saved.set(child.id, child);
  }
  return saved;
}

function assertEq(a, b, msg) {
  if (a !== b) {
    throw new Error(`assertEq failed: ${a} !== ${b}; ${msg}`);
  }
}

export function checkKeyedElements(container, saved) {
  const children = container.querySelectorAll(".keyed");
  for (const child of children) {
    console.log(`checking child=${child.outerHTML}`);
    const original = saved.get(child.id);
    if (original) {
      assertEq(original, child, `did not preserve child with key=${child.id}`);
    }
  }
}
