// `c1467.js` and `c4211.js` hash to the same id (15276222); the second probes to the next slot.
import { a } from './c1467.js';
import { b } from './c4211.js';

const required = require('./c4211.js');
globalThis.__rollipop_module_id_collision = import('./c1467.js').then((ns) => ({
  a,
  b,
  required: required.b,
  dynamic: ns.a,
}));
