// Nothing imports `sharedE` or `sharedF` from this barrel, but direct.js imports
// the same symbols from their own modules.
export { sharedE } from './register_e.js';
import { sharedF } from './register_f.js';
export { sharedF };
export const usedE = 'e';
