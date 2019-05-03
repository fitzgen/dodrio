export function initStrace() {
  instrumentAll(Element.prototype);
  instrumentAll(Document.prototype);
  instrumentAll(Node.prototype);
  instrumentAll(EventTarget.prototype);
}

const counts = new Map;
counts.total = 0;

let timer = null;

function log(ctorName, kind, key) {
  counts.total += 1;

  const entry = `${kind} ${ctorName}#${key}`
  const count = counts.get(entry);
  if (count === undefined) {
    counts.set(entry, 1);
  } else {
    counts.set(entry, count + 1);
  }

  if (timer !== null) {
    clearTimeout(timer);
  }
  timer = setTimeout(dumpAndReset, 100);
}

function dumpAndReset() {
  const data = [...counts]
        .sort((a, b) => b[1] - a[1])
        .map(a => ({ "DOM Method": a[0], "Count": a[1] }));

  data.push({ "DOM Method": "<total>", "Count": counts.total });

  console.table(data, ["DOM Method", "Count"]);

  counts.clear();
  counts.total = 0;
}

function instrument(proto, key, desc) {
  const ctorName = proto.constructor.name;

  if (typeof desc.value == "function") {
    const f = desc.value;
    desc.value = function (...args) {
      log(ctorName, "call", key);
      return f.apply(this, args);
    };
  }

  if (typeof desc.get == "function") {
    const get = desc.get;
    desc.get = function (...args) {
      log(ctorName, "get", key);
      return get.apply(this, args);
    };
  }

  if (typeof desc.set == "function") {
    const set = desc.set;
    desc.set = function (...args) {
      log(ctorName, "set", key);
      return set.apply(this, args);
    };
  }

  Object.defineProperty(proto, key, desc);
}

function instrumentAll(proto) {
  for (const key of Object.keys(proto)) {
    const desc = Object.getOwnPropertyDescriptor(proto, key);
    if (desc.configurable) {
      instrument(proto, key, desc);
    }
  }
}
