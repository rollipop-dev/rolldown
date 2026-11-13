// A side-effect import next to a named import of the same specifier, as in
// `import 'react-native-gesture-handler'` plus `import { GestureHandlerRootView }`.
// Tree-shaking merges the two declarations; the explicit side-effect import must
// still keep the module eager.
import './polyfill.js';
import { setupPolyfill } from './polyfill.js';
import './registry.js';
import registryDefault from './registry.js';
import { barrelValue, setupRanWithBarrel, setupValue as viaBarrel } from './barrel.js';

globalThis.__rollipop_merged_side_effect_result = {
  polyfill: typeof globalThis.__rollipop_merged_polyfill,
  registered: globalThis.__rollipop_merged_registered === true,
  barrelValue,
  setupRanWithBarrel,
};

// Reads the barrel's re-export later, so the barrel's getter is a real read.
export const later = () => [setupPolyfill, registryDefault, viaBarrel];
