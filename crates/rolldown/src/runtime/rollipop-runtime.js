var __rollipop_module_cache = {};
function __rollipop_require__(id) {
  var cached = __rollipop_module_cache[id];
  if (cached !== undefined) {
    if (cached.hasError) throw cached.error;
    return cached.exports;
  }
  var factory = __rollipop_modules__[id];
  if (factory === undefined) throw new Error('Module ' + id + ' is not registered');
  var module = __rollipop_module_cache[id] = { id: id, loaded: false, exports: {} };
  try {
    factory.call(module.exports, global, module, module.exports, __rollipop_require__);
  } catch (error) {
    module.hasError = true;
    module.error = error;
    throw error;
  }
  module.loaded = true;
  return module.exports;
}
__rollipop_require__.c = __rollipop_module_cache;
__rollipop_require__.o = function(obj, prop) {
  return Object.prototype.hasOwnProperty.call(obj, prop);
};
__rollipop_require__.d = function(exports, getters) {
  for (var key in getters) {
    if (__rollipop_require__.o(getters, key) && !__rollipop_require__.o(exports, key)) {
      Object.defineProperty(exports, key, { get: getters[key], enumerable: true });
    }
  }
};
__rollipop_require__.r = function(exports) {
  if (typeof Symbol !== 'undefined' && Symbol.toStringTag) {
    Object.defineProperty(exports, Symbol.toStringTag, { value: 'Module' });
  }
};
__rollipop_require__.t = function(mod, nodeMode, target) {
  var proto = mod != null ? Object.getPrototypeOf(mod) : Object.prototype;
  if (target === undefined) target = Object.create(proto);
  else Object.setPrototypeOf(target, proto);
  if (nodeMode || !mod || !mod.__esModule) Object.defineProperty(target, 'default', { value: mod, enumerable: true });
  if (mod != null) {
    var keys = Object.getOwnPropertyNames(mod);
    for (var i = 0; i < keys.length; i++) {
      var key = keys[i];
      if (__rollipop_require__.o(target, key)) continue;
      (function(k) {
        var desc = Object.getOwnPropertyDescriptor(mod, k);
        Object.defineProperty(target, k, { get: function() { return mod[k] }, enumerable: !desc || desc.enumerable });
      })(key);
    }
  }
  return target;
};
__rollipop_require__.i = function(id, nodeMode, target) {
  var mod = __rollipop_require__(id);
  var module = __rollipop_module_cache[id];
  // A module that is still initializing (circular import) can gain exports later.
  if (module === undefined || module.loaded !== true) return __rollipop_require__.t(mod, nodeMode);
  var keyCount = mod != null ? Object.getOwnPropertyNames(mod).length : 0;
  if (module.nsSource !== mod) {
    module.nsSource = mod;
    module.ns = module.nodeNs = undefined;
  } else if (module.nsKeyCount !== keyCount) {
    // Keys added after load (an `init()` or a monkeypatch) must reach later importers.
    // `.t` skips keys the target already has, so this extends the shared objects in place.
    // A frozen or sealed namespace can't take keys; later importers get a new one.
    if (module.ns) {
      if (Object.isExtensible(module.ns)) __rollipop_require__.t(mod, 0, module.ns);
      else module.ns = undefined;
    }
    if (module.nodeNs) {
      if (Object.isExtensible(module.nodeNs)) __rollipop_require__.t(mod, 1, module.nodeNs);
      else module.nodeNs = undefined;
    }
  }
  module.nsKeyCount = keyCount;
  return nodeMode
    ? module.nodeNs || (module.nodeNs = __rollipop_require__.t(mod, nodeMode, target))
    : module.ns || (module.ns = __rollipop_require__.t(mod, nodeMode, target));
};
__rollipop_require__.re = function(exports, ns) {
  for (var key in ns) {
    if (key === 'default' || key === '__esModule' || __rollipop_require__.o(exports, key)) continue;
    (function(k) {
      Object.defineProperty(exports, k, { get: function() { return ns[k] }, enumerable: true });
    })(key);
  }
};
__rollipop_require__.e = function(id) {
  throw new Error('External module ' + id + ' is not available');
};
