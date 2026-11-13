import { log } from './log.js';
import { a } from './cycle_a.js';
log('cycle_b');
export const b = 'b';
export function readA() {
  return a;
}
