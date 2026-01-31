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
// @ts-check
import { __exportAll, __reExport, __toCommonJS, __toESM } from '\0rolldown/runtime.js';
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
*/
/** @type {typeof import('./runtime-extra-dev-common-origin.js').DevRuntime} */
export var DevRuntime = /*#__PURE__*/ function() {
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
 * @type {<T>(fn: any, res: T) => () => T}
 * @internal
 */ this.createEsmInitializer = function(fn, res) {
          return function() {
              return fn && (res = fn(fn = 0)), res;
          };
      };
      /**
 * __commonJSMin
 *
 * @type {<T extends { exports: any }>(cb: any, mod: { exports: any }) => () => T}
 * @internal
 */ this.createCjsInitializer = function(cb, mod) {
          return function() {
              return mod || cb((mod = {
                  exports: {}
              }).exports, mod), mod.exports;
          };
      };
      /** @internal */ this.__toESM = __toESM;
      /** @internal */ this.__toCommonJS = __toCommonJS;
      /** @internal */ this.__exportAll = __exportAll;
      /**
 * @param {boolean} [isNodeMode]
 * @returns {(mod: any) => any}
 * @internal
 */ this.__toDynamicImportESM = function(isNodeMode) {
          return function(mod) {
              return __toESM(mod.default, isNodeMode);
          };
      };
      /** @internal */ this.__reExport = __reExport;
      this.cache = /** @type {string[]} */ [];
      this.timeout = /** @type {NodeJS.Timeout | null} */ null;
      this.timeoutSetLength = 0;
      /** @type {(module: string) => void} */ this.sendModuleRegisteredMessage = function() {
          var self = _this;
          /**
   * @param {string} module
   */ return function sendModuleRegisteredMessage(module) {
              if (!self.messenger) {
                  return;
              }
              self.cache.push(module);
              this.timeout = safetyInvokeWithSetTimeout(self.flush.bind(this));
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
      },
      {
          key: "flush",
          value: function flush() {
              if (this.cache.length > this.timeoutSetLength) {
                  this.timeoutSetLength = this.cache.length;
                  this.timeout = safetyInvokeWithSetTimeout(this.flush.bind(this));
                  return;
              }
              this.messenger.send({
                  type: 'hmr:module-registered',
                  modules: this.cache
              });
              this.cache.length = 0;
              this.timeoutSetLength = 0;
              this.timeout = null;
          }
      }
  ]);
  return DevRuntime;
}();
/**
* In lower React Native versions, `setTimeout` is cannot be used in `rolldown:hmr` initialization phase because `InitializeCore` of React Native is not evaluated yet.
* 
* `rolldown:hmr` -> `InitializeCore` -> Define polyfills (e.g, `setTimeout`)
*
* @param {() => void} fn 
*/ function safetyInvokeWithSetTimeout(fn) {
  if (typeof setTimeout === 'function') {
      return setTimeout(fn);
  }
  fn();
  return null;
}
