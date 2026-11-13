import { ns, yNs } from './barrel/index.js';
export { ns as reexportedNs } from './barrel/index.js';

const forIn = [];
for (const key in ns) forIn.push(key);
globalThis.__rollipop_lazy_namespace_binding_result = {
  keys: Object.keys(ns).sort(),
  forIn: forIn.sort(),
  spread: Object.keys({ ...yNs }).sort(),
  value: ns.value,
  same: ns === yNs,
};
