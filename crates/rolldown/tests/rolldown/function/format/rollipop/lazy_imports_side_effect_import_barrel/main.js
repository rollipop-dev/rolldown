// `import './pkg/index.js'` must run the package's whole graph at load, as it does
// eagerly, even though another file reads the package's exports through the index.
import './setup.js';
import { later } from './screen.js';

globalThis.__rollipop_side_effect_barrel_result = {
  atLoad: (globalThis.__rollipop_side_effect_barrel_log ?? []).slice(),
  later: typeof later,
};
