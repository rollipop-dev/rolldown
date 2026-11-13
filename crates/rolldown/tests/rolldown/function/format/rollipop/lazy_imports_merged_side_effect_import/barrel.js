// The same pattern inside a re-exporting barrel. Tree-shaking merges all three
// statements into `export { setupValue } from './setup.js'`.
import './setup.js';
import { setupValue } from './setup.js';
export { setupValue };
export const barrelValue = 'barrel';
export const setupRanWithBarrel = globalThis.__rollipop_merged_setup === true;
