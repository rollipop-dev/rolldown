import { actions } from './barrel.js';
import { renamed } from './reexport.js';
actions.increment();
globalThis.result = [
  Object.keys(actions).sort(),
  actions.value,
  renamed === actions,
  renamed.value,
];
