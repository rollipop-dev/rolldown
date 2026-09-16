'use strict';
import sloppy from './sloppy.js';
import strict from './strict.js';
globalThis.result = [
  (function () {
    return this === undefined;
  })(),
  sloppy(),
  strict(),
];
