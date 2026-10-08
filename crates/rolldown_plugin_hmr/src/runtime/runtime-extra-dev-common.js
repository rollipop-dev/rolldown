function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_without_holes(arr) {
    if (Array.isArray(arr)) return _array_like_to_array(arr);
}
function _assert_this_initialized(self) {
    if (self === void 0) throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    return self;
}
function _call_super(_this, derived, args) {
    derived = _get_prototype_of(derived);
    return _possible_constructor_return(_this, _is_native_reflect_construct() ? Reflect.construct(derived, args || [], _get_prototype_of(_this).constructor) : derived.apply(_this, args));
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) throw new TypeError("Cannot call a class as a function");
}
function _construct(Parent, args, Class) {
    if (_is_native_reflect_construct()) _construct = Reflect.construct;
    else {
        _construct = function construct(Parent, args, Class) {
            var a = [
                null
            ];
            a.push.apply(a, args);
            var Constructor = Function.bind.apply(Parent, a);
            var instance = new Constructor();
            if (Class) _set_prototype_of(instance, Class.prototype);
            return instance;
        };
    }
    return _construct.apply(null, arguments);
}
function _defineProperties(target, props) {
    for(var i = 0; i < props.length; i++){
        var descriptor = props[i];
        descriptor.enumerable = descriptor.enumerable || false;
        descriptor.configurable = true;
        if ("value" in descriptor) descriptor.writable = true;
        Object.defineProperty(target, descriptor.key, descriptor);
    }
}
function _create_class(Constructor, protoProps, staticProps) {
    if (protoProps) _defineProperties(Constructor.prototype, protoProps);
    if (staticProps) _defineProperties(Constructor, staticProps);
    return Constructor;
}
function _get_prototype_of(o) {
    _get_prototype_of = Object.setPrototypeOf ? Object.getPrototypeOf : function getPrototypeOf(o) {
        return o.__proto__ || Object.getPrototypeOf(o);
    };
    return _get_prototype_of(o);
}
function _inherits(subClass, superClass) {
    if (typeof superClass !== "function" && superClass !== null) {
        throw new TypeError("Super expression must either be null or a function");
    }
    subClass.prototype = Object.create(superClass && superClass.prototype, {
        constructor: {
            value: subClass,
            writable: true,
            configurable: true
        }
    });
    if (superClass) _set_prototype_of(subClass, superClass);
}
function _is_native_function(fn) {
    return Function.toString.call(fn).indexOf("[native code]") !== -1;
}
function _is_native_reflect_construct() {
    try {
        var result = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
    } catch (_) {}
    return (_is_native_reflect_construct = function() {
        return !!result;
    })();
}
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) {
        return Array.from(iter);
    }
}
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _possible_constructor_return(self, call) {
    if (call && (_type_of(call) === "object" || typeof call === "function")) return call;
    return _assert_this_initialized(self);
}
function _set_prototype_of(o, p) {
    _set_prototype_of = Object.setPrototypeOf || function setPrototypeOf(o, p) {
        o.__proto__ = p;
        return o;
    };
    return _set_prototype_of(o, p);
}
function _to_consumable_array(arr) {
    return _array_without_holes(arr) || _iterable_to_array(arr) || _unsupported_iterable_to_array(arr) || _non_iterable_spread();
}
function _type_of(obj) {
    "@swc/helpers - typeof";
    return obj && typeof Symbol !== "undefined" && obj.constructor === Symbol ? "symbol" : typeof obj;
}
function _unsupported_iterable_to_array(o, minLen) {
    if (!o) return;
    if (typeof o === "string") return _array_like_to_array(o, minLen);
    var n = Object.prototype.toString.call(o).slice(8, -1);
    if (n === "Object" && o.constructor) n = o.constructor.name;
    if (n === "Map" || n === "Set") return Array.from(n);
    if (n === "Arguments" || /^(?:Ui|I)nt(?:8|16|32)(?:Clamped)?Array$/.test(n)) return _array_like_to_array(o, minLen);
}
function _wrap_native_super(Class) {
    var _cache = typeof Map === "function" ? new Map() : undefined;
    _wrap_native_super = function(Class) {
        if (Class === null || !_is_native_function(Class)) return Class;
        if (typeof Class !== "function") throw new TypeError("Super expression must either be null or a function");
        if (typeof _cache !== "undefined") {
            if (_cache.has(Class)) return _cache.get(Class);
            _cache.set(Class, Wrapper);
        }
        function Wrapper() {
            return _construct(Class, arguments, _get_prototype_of(this).constructor);
        }
        Wrapper.prototype = Object.create(Class.prototype, {
            constructor: {
                value: Wrapper,
                enumerable: false,
                writable: true,
                configurable: true
            }
        });
        return _set_prototype_of(Wrapper, Class);
    };
    return _wrap_native_super(Class);
}
// @ts-check
var Module = /*#__PURE__*/ function() {
    "use strict";
    function Module(id) {
        _class_call_check(this, Module);
        /**
   * @type {{ exports: any }}
   */ this.exportsHolder = {
            exports: null
        };
        this.id = id;
    }
    _create_class(Module, [
        {
            key: "exports",
            get: function get() {
                return this.exportsHolder.exports;
            }
        }
    ]);
    return Module;
}();
/**
 * Compiler-emitted module-graph delta — topology (static + dynamic edges).
 * `ids[0, localCount)` are the modules this payload carries; `ids[localCount, …)` are foreign edge targets.
 * `edges[i]` / `dynamicEdges[i]` are the static / dynamic-`import()` out-edges of `ids[i]`.
 * `bindings[i][j]` are the export names `ids[i]` imports through `edges[i][j]`. `bindings` holds
 * only some rows; a missing row, or a missing or `null` entry, means the whole namespace.
 * `dynamicEdges` also holds only some rows; a missing row means no dynamic edges.
 * `stamps[i]` is the rebuild stamp of `ids[i]`; a missing row means 0.
 * @typedef {{ ids: string[], localCount: number, edges: number[][], bindings?: Record<number, (string[] | null)[]>, dynamicEdges?: Record<number, number[]>, stamps?: Record<number, number> }} ModuleGraphDelta
 * @typedef {{ createModuleHotContext(moduleId: string): any, onModuleCacheRemoval(moduleId: string): void }} DevRuntimeHooks
 */ export var MissingFactoryError = /*#__PURE__*/ function(Error1) {
    "use strict";
    _inherits(MissingFactoryError, Error1);
    function MissingFactoryError(id) {
        _class_call_check(this, MissingFactoryError);
        var _this;
        _this = _call_super(this, MissingFactoryError, [
            "No factory registered for module ".concat(id)
        ]);
        _this.id = id;
        return _this;
    }
    return MissingFactoryError;
}(_wrap_native_super(Error));
export var DevRuntime = /*#__PURE__*/ function() {
    "use strict";
    function DevRuntime(clientId) {
        _class_call_check(this, DevRuntime);
        /**
   * Static import edges from `registerGraph` — entries persist across `removeModuleCache`
   * and change only by replacement from a newer payload (last write wins).
   * `bindings` is the payload row as sent; `getImportedBindings` resolves a missing entry.
   * @type {Map<string, { edges: string[], bindings: (string[] | null)[] | undefined }>}
   */ this.staticImports = new Map();
        /**
   * Reverse index over the static imports.
   * @type {Map<string, Set<string>>}
   */ this.importers = new Map();
        /**
   * Dynamic `import()` edges from `registerGraph`, keyed by importer — mirror of
   * `staticImports` for the dynamic reverse index.
   * @type {Map<string, { edges: string[] }>}
   */ this.dynamicImports = new Map();
        /**
   * Reverse index over the dynamic imports.
   * @type {Map<string, Set<string>>}
   */ this.dynamicImporters = new Map();
        /**
   * The module cache. Membership means "this module's side effects ran in this tab" —
   * registration is emitted ahead of every module body, and nothing un-registers on
   * unwind, so a factory that throws mid-body stays registered. A `Map` rather than a
   * plain object: HMR eviction deletes entries, and a `delete` on an object drops V8
   * into dictionary mode, taxing every later lookup on the hottest read path.
   * @type {Map<string, Module>}
   */ this.moduleCache = new Map();
        /**
   * Re-runnable factories from HMR patches and lazy chunks. The initial bundle stays
   * scope-hoisted and contributes none.
   * @type {Map<string, (id: string) => void>}
   */ this.factories = new Map();
        /**
   * A late payload (a slow lazy chunk after an HMR patch) must not replace newer rows or
   * factories.
   * @type {Map<string, number>}
   */ this.rowStamps = new Map();
        /** @type {Map<string, number>} */ this.factoryStamps = new Map();
        /**
   * Installed by the dev client at boot. The runtime is a store + executor and makes
   * no HMR decisions; accepting, disposing, and reloading live behind these hooks.
   * @type {DevRuntimeHooks | null}
   */ this.hooks = null;
        /** @internal */ // @ts-expect-error The variable will be injected at build time.
        this.__toESM = __toESM;
        /** @internal */ // @ts-expect-error The variable will be injected at build time.
        this.__toCommonJS = __toCommonJS;
        /** @internal */ // @ts-expect-error The variable will be injected at build time.
        this.__exportAll = __exportAll;
        /**
   * @param {boolean} [isNodeMode]
   * @returns {(mod: any) => any}
   * @internal
   */ // @ts-expect-error The variable will be injected at build time.
        this.__toDynamicImportESM = function(isNodeMode) {
            return function(mod) {
                return __toESM(mod.default, isNodeMode);
            };
        };
        /** @internal */ // @ts-expect-error The variable will be injected at build time.
        this.__reExport = __reExport;
        this.clientId = clientId;
    }
    _create_class(DevRuntime, [
        {
            /**
   * @param {ModuleGraphDelta} delta
   */ key: "registerGraph",
            value: function registerGraph(delta) {
                for(var i = 0; i < delta.localCount; i++){
                    var _ref, _ref1, _ref2;
                    var _this_staticImports_get, _delta_bindings, _delta_dynamicEdges, _this_dynamicImports_get;
                    var id = delta.ids[i];
                    if (delta.stamps) {
                        var _delta_stamps_i, _this_rowStamps_get;
                        var stamp = (_delta_stamps_i = delta.stamps[i]) !== null && _delta_stamps_i !== void 0 ? _delta_stamps_i : 0;
                        if (stamp < ((_this_rowStamps_get = this.rowStamps.get(id)) !== null && _this_rowStamps_get !== void 0 ? _this_rowStamps_get : 0)) continue;
                        this.rowStamps.set(id, stamp);
                    } else {
                        // A full-build chunk has no stamps. It runs its code right away, so its row always
                        // replaces the old one.
                        this.rowStamps.delete(id);
                    }
                    var edges = delta.edges[i].map(function(j) {
                        return delta.ids[j];
                    });
                    var _iteratorNormalCompletion = true, _didIteratorError = false, _iteratorError = undefined;
                    try {
                        for(var _iterator = ((_ref = (_this_staticImports_get = this.staticImports.get(id)) === null || _this_staticImports_get === void 0 ? void 0 : _this_staticImports_get.edges) !== null && _ref !== void 0 ? _ref : [])[Symbol.iterator](), _step; !(_iteratorNormalCompletion = (_step = _iterator.next()).done); _iteratorNormalCompletion = true){
                            var target = _step.value;
                            var _this_importers_get;
                            (_this_importers_get = this.importers.get(target)) === null || _this_importers_get === void 0 ? void 0 : _this_importers_get.delete(id);
                        }
                    } catch (err) {
                        _didIteratorError = true;
                        _iteratorError = err;
                    } finally{
                        try {
                            if (!_iteratorNormalCompletion && _iterator.return != null) {
                                _iterator.return();
                            }
                        } finally{
                            if (_didIteratorError) {
                                throw _iteratorError;
                            }
                        }
                    }
                    var _iteratorNormalCompletion1 = true, _didIteratorError1 = false, _iteratorError1 = undefined;
                    try {
                        for(var _iterator1 = edges[Symbol.iterator](), _step1; !(_iteratorNormalCompletion1 = (_step1 = _iterator1.next()).done); _iteratorNormalCompletion1 = true){
                            var target1 = _step1.value;
                            var importerSet = this.importers.get(target1);
                            if (!importerSet) {
                                importerSet = new Set();
                                this.importers.set(target1, importerSet);
                            }
                            importerSet.add(id);
                        }
                    } catch (err) {
                        _didIteratorError1 = true;
                        _iteratorError1 = err;
                    } finally{
                        try {
                            if (!_iteratorNormalCompletion1 && _iterator1.return != null) {
                                _iterator1.return();
                            }
                        } finally{
                            if (_didIteratorError1) {
                                throw _iteratorError1;
                            }
                        }
                    }
                    this.staticImports.set(id, {
                        edges: edges,
                        bindings: (_delta_bindings = delta.bindings) === null || _delta_bindings === void 0 ? void 0 : _delta_bindings[i]
                    });
                    // Dynamic `import()` edges are maintained in a parallel reverse index with the same
                    // last-write-wins bookkeeping; `getImporters` unions the two.
                    var dynamicEdges = ((_ref1 = (_delta_dynamicEdges = delta.dynamicEdges) === null || _delta_dynamicEdges === void 0 ? void 0 : _delta_dynamicEdges[i]) !== null && _ref1 !== void 0 ? _ref1 : []).map(function(j) {
                        return delta.ids[j];
                    });
                    var _iteratorNormalCompletion2 = true, _didIteratorError2 = false, _iteratorError2 = undefined;
                    try {
                        for(var _iterator2 = ((_ref2 = (_this_dynamicImports_get = this.dynamicImports.get(id)) === null || _this_dynamicImports_get === void 0 ? void 0 : _this_dynamicImports_get.edges) !== null && _ref2 !== void 0 ? _ref2 : [])[Symbol.iterator](), _step2; !(_iteratorNormalCompletion2 = (_step2 = _iterator2.next()).done); _iteratorNormalCompletion2 = true){
                            var target2 = _step2.value;
                            var _this_dynamicImporters_get;
                            (_this_dynamicImporters_get = this.dynamicImporters.get(target2)) === null || _this_dynamicImporters_get === void 0 ? void 0 : _this_dynamicImporters_get.delete(id);
                        }
                    } catch (err) {
                        _didIteratorError2 = true;
                        _iteratorError2 = err;
                    } finally{
                        try {
                            if (!_iteratorNormalCompletion2 && _iterator2.return != null) {
                                _iterator2.return();
                            }
                        } finally{
                            if (_didIteratorError2) {
                                throw _iteratorError2;
                            }
                        }
                    }
                    var _iteratorNormalCompletion3 = true, _didIteratorError3 = false, _iteratorError3 = undefined;
                    try {
                        for(var _iterator3 = dynamicEdges[Symbol.iterator](), _step3; !(_iteratorNormalCompletion3 = (_step3 = _iterator3.next()).done); _iteratorNormalCompletion3 = true){
                            var target3 = _step3.value;
                            var importerSet1 = this.dynamicImporters.get(target3);
                            if (!importerSet1) {
                                importerSet1 = new Set();
                                this.dynamicImporters.set(target3, importerSet1);
                            }
                            importerSet1.add(id);
                        }
                    } catch (err) {
                        _didIteratorError3 = true;
                        _iteratorError3 = err;
                    } finally{
                        try {
                            if (!_iteratorNormalCompletion3 && _iterator3.return != null) {
                                _iterator3.return();
                            }
                        } finally{
                            if (_didIteratorError3) {
                                throw _iteratorError3;
                            }
                        }
                    }
                    this.dynamicImports.set(id, {
                        edges: dynamicEdges
                    });
                }
            }
        },
        {
            /**
   * @param {string} id
   * @param {(id: string) => void} fn
   * @param {number} [stamp]
   */ key: "registerFactory",
            value: function registerFactory(id, fn) {
                var stamp = arguments.length > 2 && arguments[2] !== void 0 ? arguments[2] : 0;
                var _this_factoryStamps_get;
                if (stamp < ((_this_factoryStamps_get = this.factoryStamps.get(id)) !== null && _this_factoryStamps_get !== void 0 ? _this_factoryStamps_get : 0)) return;
                this.factoryStamps.set(id, stamp);
                this.factories.set(id, fn);
            }
        },
        {
            /**
   * @param {string} id
   * @param {{ exports: any }} [exportsHolder]
   */ key: "registerModule",
            value: function registerModule(id) {
                var exportsHolder = arguments.length > 1 && arguments[1] !== void 0 ? arguments[1] : {
                    exports: {}
                };
                var module = new Module(id);
                module.exportsHolder = exportsHolder;
                this.moduleCache.set(id, module);
            }
        },
        {
            /**
   * @param {string} id
   * @returns {string[]}
   */ key: "getImporters",
            value: function getImporters(id) {
                var _this_importers_get;
                // Static ∪ dynamic importers — the boundary walk treats both kinds the same (parity
                // with Vite `node.importers` / webpack `module.parents`). Deduped so a module that
                // imports `id` both statically and via `import()` appears once.
                var dynamic = this.dynamicImporters.get(id);
                if (!dynamic || dynamic.size === 0) {
                    var _this_importers_get1;
                    return _to_consumable_array((_this_importers_get1 = this.importers.get(id)) !== null && _this_importers_get1 !== void 0 ? _this_importers_get1 : []);
                }
                return _to_consumable_array(new Set(_to_consumable_array((_this_importers_get = this.importers.get(id)) !== null && _this_importers_get !== void 0 ? _this_importers_get : []).concat(_to_consumable_array(dynamic))));
            }
        },
        {
            /**
   * `"*"` means the whole namespace, an empty list a side-effect-only import, `undefined` no edge.
   * @param {string} importer
   * @param {string} id
   * @returns {string[] | undefined}
   */ key: "getImportedBindings",
            value: function getImportedBindings(importer, id) {
                var _ref;
                var _record_bindings, _this_dynamicImports_get;
                var record = this.staticImports.get(importer);
                var j = record ? record.edges.indexOf(id) : -1;
                var names = j === -1 ? undefined : (_ref = record === null || record === void 0 ? void 0 : (_record_bindings = record.bindings) === null || _record_bindings === void 0 ? void 0 : _record_bindings[j]) !== null && _ref !== void 0 ? _ref : [
                    '*'
                ];
                if (!((_this_dynamicImports_get = this.dynamicImports.get(importer)) === null || _this_dynamicImports_get === void 0 ? void 0 : _this_dynamicImports_get.edges.includes(id))) {
                    return names;
                }
                if (!names) return [
                    '*'
                ];
                return names.includes('*') ? names : _to_consumable_array(names).concat([
                    '*'
                ]);
            }
        },
        {
            /**
   * @param {string} id
   */ key: "isExecuted",
            value: function isExecuted(id) {
                return this.moduleCache.has(id);
            }
        },
        {
            /**
   * @param {string} id
   */ key: "hasFactory",
            value: function hasFactory(id) {
                return this.factories.has(id);
            }
        },
        {
            /**
   * Module-cache delete only — static imports and factories persist. Removal is what
   * re-arms a cache-gated factory for `initModule`.
   * @param {string} id
   */ key: "removeModuleCache",
            value: function removeModuleCache(id) {
                var _this_hooks;
                this.moduleCache.delete(id);
                (_this_hooks = this.hooks) === null || _this_hooks === void 0 ? void 0 : _this_hooks.onModuleCacheRemoval(id);
            }
        },
        {
            /**
   * The one re-execution gate: registered → return the live exports; otherwise run the
   * mapped factory (which registers itself first, then runs the body).
   * @param {string} id
   */ key: "initModule",
            value: function initModule(id) {
                if (this.moduleCache.has(id)) {
                    return this.loadExports(id);
                }
                var factory = this.factories.get(id);
                if (!factory) {
                    throw new MissingFactoryError(id);
                }
                factory(id);
                return this.loadExports(id);
            }
        },
        {
            /**
   * @param {string} id
   */ key: "loadExports",
            value: function loadExports(id) {
                var module = this.moduleCache.get(id);
                if (module) {
                    return module.exportsHolder.exports;
                } else {
                    console.warn("Module ".concat(id, " not found"));
                    return {};
                }
            }
        },
        {
            /**
   * @param {string} moduleId
   */ key: "createModuleHotContext",
            value: function createModuleHotContext(moduleId) {
                if (this.hooks) {
                    return this.hooks.createModuleHotContext(moduleId);
                }
                throw new Error('createModuleHotContext requires installed hooks or an override');
            }
        }
    ]);
    return DevRuntime;
}();
