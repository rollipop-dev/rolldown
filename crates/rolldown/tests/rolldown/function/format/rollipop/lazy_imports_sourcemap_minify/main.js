import './side.js';
import { lazyValue, lazyThrow } from './lazy.js';
import { cjsValue } from './cjs.cjs';

export function run() {
  return [lazyValue, cjsValue, 'main-marker', lazyThrow];
}
globalThis.__rollipop_sourcemap_run = run;
