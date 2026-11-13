// Interop fails immediately: reading `__esModule` throws.
Object.defineProperty(exports, '__esModule', {
  get() {
    throw new Error('esmodule-flag');
  },
});
exports.flagged = 'flagged';
