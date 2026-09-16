import fn, { object, Constructor } from './dep.js';
import * as ns from './dep.js';
globalThis.result = [
  fn() === undefined,
  fn?.() === undefined,
  fn`tag` === undefined,
  fn() === undefined,
  ns.default() === ns,
  ns.default?.() === ns,
  ns.default`tag` === ns,
  object.method() === object,
  new Constructor().value,
];
