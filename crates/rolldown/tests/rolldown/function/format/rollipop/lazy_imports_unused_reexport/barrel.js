// `registered` is never imported, so only register_a.js's side effect is left.
export { registered } from './register_a.js';
export const used = 'a';
