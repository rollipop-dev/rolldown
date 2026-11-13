var __rollipop_lazy_namespaces = [{}, {}, {}];
function __rollipop_lazy_load(id, mode) {
  return mode === 2 ? __rollipop_require__(id) : __rollipop_require__.i(id, mode);
}
function __rollipop_lazy_resolve(state) {
  var target = state.target;
  // CommonJS namespaces are built directly on the facade when it is the first one.
  var ns = state.mode === 2
    ? __rollipop_require__(state.id)
    : __rollipop_require__.i(state.id, state.mode, target);
  if (ns !== target && __rollipop_module_cache[state.id].loaded === true) {
    Object.setPrototypeOf(target, Object.getPrototypeOf(ns));
    var keys = Object.getOwnPropertyNames(ns);
    for (var i = 0; i < keys.length; i++) {
      Object.defineProperty(target, keys[i], Object.getOwnPropertyDescriptor(ns, keys[i]));
    }
  }
  return ns;
}
var __rollipop_lazy_handler = {
  get: function(state, key) {
    return __rollipop_lazy_resolve(state)[key];
  },
  has: function(state, key) {
    return key in __rollipop_lazy_resolve(state);
  }
};
__rollipop_require__.l = function(id, mode) {
  if (mode === undefined) mode = 2;
  if (__rollipop_module_cache[id] !== undefined || typeof Proxy !== 'function') {
    return __rollipop_lazy_load(id, mode);
  }
  var namespaces = __rollipop_lazy_namespaces[mode];
  var ns = namespaces[id];
  if (ns === undefined) {
    var state = { id: id, mode: mode, target: undefined };
    ns = namespaces[id] = state.target = Object.create(new Proxy(state, __rollipop_lazy_handler));
  }
  return ns;
};
