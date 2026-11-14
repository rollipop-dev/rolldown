// `__rollipop_require__.l(id, mode)` returns a holder whose `a` property is what the
// importee is read through; generated code reads `holder.a.name`. `mode` is omitted for
// ESM importees and for CommonJS importees read only through named exports (`a` is then
// `module.exports`), `0` for Babel interop and `1` for Node interop.
// See internal-docs/rollipop-lazy-imports/implementation.md
function __rollipop_lazy_load(id, mode) {
  return mode === undefined ? __rollipop_require__(id) : __rollipop_require__.i(id, mode);
}
__rollipop_require__.l = function(id, mode) {
  var record = __rollipop_module_cache[id];
  if (record !== undefined && record.loaded === true) return { a: __rollipop_lazy_load(id, mode) };
  return {
    get a() {
      var ns = __rollipop_lazy_load(id, mode);
      // Replace the getter only once the module finished loading and interop succeeded.
      // A module still loading (a cycle) is read through to its partial exports, and a
      // throw leaves the getter in place, so every later read throws again.
      if (__rollipop_module_cache[id].loaded === true) Object.defineProperty(this, 'a', { value: ns });
      return ns;
    }
  };
};
