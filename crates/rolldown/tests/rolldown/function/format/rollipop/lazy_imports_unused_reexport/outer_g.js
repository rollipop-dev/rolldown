// Two barrels; nothing imports `sharedG` through them, but direct.js imports it directly.
export { sharedG } from './inner_g.js';
export const chainedG = 'g';
