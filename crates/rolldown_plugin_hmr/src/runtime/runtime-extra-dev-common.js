function _array_like_to_array(arr, len) {
    if (len == null || len > arr.length) len = arr.length;
    for(var i = 0, arr2 = new Array(len); i < len; i++)arr2[i] = arr[i];
    return arr2;
}
function _array_without_holes(arr) {
    if (Array.isArray(arr)) return _array_like_to_array(arr);
}
function _assert_this_initialized(self) {
    if (self === void 0) {
        throw new ReferenceError("this hasn't been initialised - super() hasn't been called");
    }
    return self;
}
function _call_super(_this, derived, args) {
    derived = _get_prototype_of(derived);
    return _possible_constructor_return(_this, _is_native_reflect_construct() ? Reflect.construct(derived, args || [], _get_prototype_of(_this).constructor) : derived.apply(_this, args));
}
function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
}
function _construct(Parent, args, Class) {
    if (_is_native_reflect_construct()) {
        _construct = Reflect.construct;
    } else {
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
function _iterable_to_array(iter) {
    if (typeof Symbol !== "undefined" && iter[Symbol.iterator] != null || iter["@@iterator"] != null) return Array.from(iter);
}
function _non_iterable_spread() {
    throw new TypeError("Invalid attempt to spread non-iterable instance.\\nIn order to be iterable, non-array objects must have a [Symbol.iterator]() method.");
}
function _possible_constructor_return(self, call) {
    if (call && (_type_of(call) === "object" || typeof call === "function")) {
        return call;
    }
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
    _wrap_native_super = function wrapNativeSuper(Class) {
        if (Class === null || !_is_native_function(Class)) return Class;
        if (typeof Class !== "function") {
            throw new TypeError("Super expression must either be null or a function");
        }
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
function _is_native_reflect_construct() {
    try {
        var result = !Boolean.prototype.valueOf.call(Reflect.construct(Boolean, [], function() {}));
    } catch (_) {}
    return (_is_native_reflect_construct = function() {
        return !!result;
    })();
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
 * Compiler-emitted module-graph delta — pure topology (static + dynamic edges).
 * `ids[0, localCount)` are the modules this payload carries; `ids[localCount, …)` are foreign edge targets.
 * `edges[i]` / `dynamicEdges[i]` are the static / dynamic-`import()` out-edges of `ids[i]`.
 * @typedef {{ ids: string[], localCount: number, edges: number[][], dynamicEdges?: number[][] }} ModuleGraphDelta
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
   * @type {Map<string, { edges: string[] }>}
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
   * @type {Map<string, { kind: 'esm' | 'cjs', fn: (id: string) => void }>}
   */ this.factories = new Map();
        /**
   * Installed by the dev client at boot. The runtime is a store + executor and makes
   * no HMR decisions; accepting, disposing, and reloading live behind these hooks.
   * @type {DevRuntimeHooks | null}
   */ this.hooks = null;
        /** @type {Map<string, Promise<any>>} */ this.lazyRequests = new Map();
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
                    var _this_staticImports_get, _delta_dynamicEdges, _this_dynamicImports_get;
                    var id = delta.ids[i];
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
                        edges: edges
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
   * @param {'esm' | 'cjs'} kind
   * @param {(id: string) => void} fn
   */ key: "registerFactory",
            value: function registerFactory(id, kind, fn) {
                this.factories.set(id, {
                    kind: kind,
                    fn: fn
                });
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
                // `registerModule` installs a fresh namespace on every run, so a kept memo would hand a
                // later `import()` the pre-edit exports forever.
                this.lazyRequests.delete(id);
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
                factory.fn(id);
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
   * The entry point for a lazy `import()`. `id` is the module the boundary stands for; the
   * boundary's own id appears only inside `fetchChunk`'s URL.
   *
   * Nothing is registered under the boundary id, deliberately. A cache entry with no factory
   * behind it reads as "executed" to the HMR boundary walk, and `applyUpdate` turns an
   * updated-but-factory-less module into a full page reload.
   *
   * A rejection is memoized like any other outcome, matching `import()` of a module that
   * threw. Retrying could not work anyway: a factory registers its module before running its
   * body, so re-running `initModule` would return half-initialized exports as success.
   *
   * @param {string} id
   * @param {() => Promise<unknown>} fetchChunk
   * @returns {Promise<any>}
   */ key: "requestLazy",
            value: function requestLazy(id, fetchChunk) {
                var _this = this;
                var pending = this.lazyRequests.get(id);
                if (pending) {
                    return pending;
                }
                // Factories outlive `removeModuleCache`, so an evicted module can be re-run without the
                // server — and must be, since the memo went with the cache entry. `Promise.resolve` keeps
                // a synchronous `initModule` throw from escaping the call site.
                var runnableHere = this.moduleCache.has(id) || this.factories.has(id);
                var promise = runnableHere ? Promise.resolve().then(function() {
                    return _this.initModule(id);
                }) : Promise.resolve().then(fetchChunk).then(function() {
                    return _this.initModule(id);
                });
                this.lazyRequests.set(id, promise);
                return promise;
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
