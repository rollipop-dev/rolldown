import { log } from './log.js';
log('lazy');
export const lazyValue = 'lazy-value';
export default function lazyDefault() {
  return 'lazy-default';
}
