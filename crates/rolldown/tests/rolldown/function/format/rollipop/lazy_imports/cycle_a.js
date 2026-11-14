import { log } from './log.js';
import { b } from './cycle_b.js';
log('cycle_a');
export const a = 'a';
export function readB() {
  return b;
}
