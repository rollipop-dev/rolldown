import { top, arrow, receiver } from './esm.js';
const cjs = require('./dep.cjs');
globalThis.result = [
  top === undefined,
  arrow() === undefined,
  receiver.call(42) === 42,
  cjs.top === cjs,
  cjs.arrow() === cjs,
  (0, cjs.sloppy)() === globalThis,
];
