// The same pattern inside a re-exporting barrel.
import './setup.js';
import { setupValue } from './setup.js';
export { setupValue };
export const barrelValue = 'barrel';
