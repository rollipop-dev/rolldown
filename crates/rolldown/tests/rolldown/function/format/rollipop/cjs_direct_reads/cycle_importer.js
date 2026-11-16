import { early, late } from './cycle.cjs';
globalThis.__rollipop_cjs_direct_cycle = { early, lateDuringLoad: late };
export function readLate() {
  return late;
}
