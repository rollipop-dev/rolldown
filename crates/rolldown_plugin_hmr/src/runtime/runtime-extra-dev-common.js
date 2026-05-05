function _class_call_check(instance, Constructor) {
    if (!(instance instanceof Constructor)) {
        throw new TypeError("Cannot call a class as a function");
    }
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
 * @typedef {{ type: 'hmr:module-registered', modules: string[] }} DevRuntimeMessage
 * @typedef {{ send(message: DevRuntimeMessage): void }} Messenger
 */ export var DevRuntime = /*#__PURE__*/ function() {
    "use strict";
    function DevRuntime(messenger, clientId) {
        var _this = this;
        _class_call_check(this, DevRuntime);
        /**
   * @type {Record<string, Module>}
   */ this.modules = {};
        /**
   * __esmMin
   *
   * When `dedup` is truthy and `id` is already registered on the runtime,
   * skip the factory: another lazy blob got there first. HMR patches pass
   * no `dedup` so they always re-run the factory and replace the registered
   * exports.
   *
   * @type {<T>(id: string, fn: any, dedup: any, res: T) => () => T}
   * @internal
   */ this.createEsmInitializer = function(id, fn, dedup, res) {
            return function() {
                return fn && (dedup && _this.modules[id] ? fn = 0 : res = fn((fn = 0, id))), res;
            };
        };
        /**
   * __commonJSMin
   *
   * Same dedup gate as createEsmInitializer. With `dedup` truthy and `id`
   * registered, reuse the registered exports object; otherwise run the
   * factory.
   *
   * @type {<T extends { exports: any }>(id: string, cb: any, dedup: any, mod: { exports: any }, registered: any) => () => T}
   * @internal
   */ this.createCjsInitializer = function(id, cb, dedup, mod, registered) {
            return function() {
                return mod || (dedup && (registered = _this.modules[id]) ? mod = {
                    exports: registered.exports
                } : cb((mod = {
                    exports: {}
                }).exports, mod, id)), mod.exports;
            };
        };
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
        this.sendModuleRegisteredMessage = function() {
            var cache = /** @type {string[]} */ [];
            var scheduled = /** @type {boolean} */ false;
            var timeoutSetLength = 0;
            var self = _this;
            /**
     * @param {string} module
     */ return function sendModuleRegisteredMessage(module) {
                if (!self.messenger) {
                    return;
                }
                cache.push(module);
                if (!scheduled) {
                    timeoutSetLength = cache.length;
                    scheduled = __schedule(/** @returns void */ function flushCache() {
                        if (cache.length > timeoutSetLength) {
                            timeoutSetLength = cache.length;
                            scheduled = __schedule(flushCache);
                            return;
                        }
                        self.messenger.send({
                            type: 'hmr:module-registered',
                            modules: cache
                        });
                        cache.length = 0;
                        scheduled = false;
                        timeoutSetLength = 0;
                    });
                }
            };
        }();
        this.messenger = messenger;
        this.clientId = clientId;
    }
    _create_class(DevRuntime, [
        {
            /**
   * @param {string} _moduleId
   */ key: "createModuleHotContext",
            value: function createModuleHotContext(_moduleId) {
                throw new Error('createModuleHotContext should be implemented');
            }
        },
        {
            /**
   * @param {[string, string][]} _boundaries
   */ key: "applyUpdates",
            value: function applyUpdates(_boundaries) {
                throw new Error('applyUpdates should be implemented');
            }
        },
        {
            /**
   * @param {string} id
   * @param {{ exports: any }} exportsHolder
   */ key: "registerModule",
            value: function registerModule(id, exportsHolder) {
                var module = new Module(id);
                module.exportsHolder = exportsHolder;
                this.modules[id] = module;
                this.sendModuleRegisteredMessage(id);
            }
        },
        {
            /**
   * @param {string} id
   */ key: "loadExports",
            value: function loadExports(id) {
                var module = this.modules[id];
                if (module) {
                    return module.exportsHolder.exports;
                } else {
                    console.warn("Module ".concat(id, " not found"));
                    return {};
                }
            }
        }
    ]);
    return DevRuntime;
}();
/**
 * @param {() => void} callback
 */ function __schedule(callback) {
    if (typeof setTimeout === 'function') {
        setTimeout(callback, 0);
        return true;
    }
    return false;
}


