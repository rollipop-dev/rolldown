import data, { version } from './data.json';
import mutable from './mutable.json';
import escaped from './escaped.json';
import split from './split.json';

globalThis.escaped = escaped;
mutable.value += 1;
globalThis.result = {
  split: split.name,
  splitNested: split.nested.value,
  name: data.name,
  version,
  nested: data.nested.value,
  quoted: data['dashed-key'],
  defaultField: data.default,
  missing: data.missing,
  mutable: mutable.value,
  escaped: globalThis.escaped,
  array: require('./array.json'),
  primitive: require('./primitive.json'),
};
