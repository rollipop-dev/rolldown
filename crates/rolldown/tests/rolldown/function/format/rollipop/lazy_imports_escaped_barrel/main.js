// Each package's index is loaded in a way that hands out its whole namespace:
// `require()`, `import()`, an escaping `import * as`, and `export * as`. The
// modules it re-exports must run with it, as they do eagerly, even though
// screen.js reads them through the index later.
import './setup_require.cjs';
import './setup_escape.js';
import { reexportValue } from './reexport_namespace.js';
import { later } from './screen.js';

// Read at load, so reexport_namespace.js (and the namespace it re-exports) runs now.
const reexportAtLoad = reexportValue;
const atLoad = (globalThis.__rollipop_escaped_barrel_log ?? []).slice();
globalThis.__rollipop_escaped_barrel_result = import('./dyn/index.js').then(() => ({
  atLoad,
  afterDynamicImport: (globalThis.__rollipop_escaped_barrel_log ?? []).slice(),
  reexportValue: reexportAtLoad,
  later: typeof later,
}));
