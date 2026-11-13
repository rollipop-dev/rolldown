import './side.js';
import { unusedEffect } from './effect.js';
import * as ns from './namespace.js';
import { fromStar, reexportedLazy } from './reexport.js';
import lazyDefault, { lazyValue } from './lazy.js';
import { neverValue } from './never.js';
import cjs, { named } from './cjs.cjs';
import { unreachable } from './throws.js';
import { readB } from './cycle_a.js';
import { log } from './log.js';

log('main');
const result = {};
result.beforeAccess = globalThis.__rollipop_lazy_log.slice();
result.lazyValue = lazyValue;
result.lazyDefault = lazyDefault();
result.afterLazy = globalThis.__rollipop_lazy_log.slice();
result.namespace = ns.fromNamespace;
result.fromStar = fromStar;
result.reexportedLazy = reexportedLazy;
result.cjsDefault = cjs.named;
result.cjsNamed = named;
result.cycle = readB();
const errors = [];
for (let i = 0; i < 2; i++) {
  try {
    errors.push(unreachable);
  } catch (error) {
    errors.push(error.message);
  }
}
result.errors = errors;
result.final = globalThis.__rollipop_lazy_log.slice();
globalThis.__rollipop_lazy_imports_result = result;

export function useNever() {
  return neverValue;
}
