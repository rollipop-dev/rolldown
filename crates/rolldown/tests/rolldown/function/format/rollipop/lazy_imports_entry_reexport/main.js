// Nothing in the bundle reads an entry's exports, so a lazy re-export here would never run.
export { setup } from './setup.js';
import { other } from './other.js';
export { other };
