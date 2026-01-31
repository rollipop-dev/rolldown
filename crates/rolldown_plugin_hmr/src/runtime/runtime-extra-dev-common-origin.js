// @ts-check
import {
  __exportAll,
  __reExport,
  __toCommonJS,
  __toESM,
  // @ts-expect-error
} from '\0rolldown/runtime.js';

class Module {
  /**
   * @type {{ exports: any }}
   */
  exportsHolder = { exports: null };
  /**
   * @type {string}
   */
  id;

  /**
   * @param {string} id
   */
  constructor(id) {
    this.id = id;
  }

  get exports() {
    return this.exportsHolder.exports;
  }
}

/**
 * @typedef {{ type: 'hmr:module-registered', modules: string[] }} DevRuntimeMessage
 * @typedef {{ send(message: DevRuntimeMessage): void }} Messenger
 */

export class DevRuntime {
  /**
   * Client ID generated at runtime initialization, used for lazy compilation requests.
   * @type {string}
   */
  clientId;

  /**
   * @param {Messenger} messenger
   * @param {string} clientId
   */
  constructor(messenger, clientId) {
    this.messenger = messenger;
    this.clientId = clientId;
  }

  /**
   * @type {Record<string, Module>}
   */
  modules = {};
  /**
   * @param {string} _moduleId
   */
  createModuleHotContext(_moduleId) {
    throw new Error('createModuleHotContext should be implemented');
  }
  /**
   * @param {[string, string][]} _boundaries
   */
  applyUpdates(_boundaries) {
    throw new Error('applyUpdates should be implemented');
  }
  /**
   * @param {string} id
   * @param {{ exports: any }} exportsHolder
   */
  registerModule(id, exportsHolder) {
    const module = new Module(id);
    module.exportsHolder = exportsHolder;
    this.modules[id] = module;
    this.sendModuleRegisteredMessage(id);
  }
  /**
   * @param {string} id
   */
  loadExports(id) {
    const module = this.modules[id];
    if (module) {
      return module.exportsHolder.exports;
    } else {
      console.warn(`Module ${id} not found`);
      return {};
    }
  }

  /**
   * __esmMin
   *
   * @type {<T>(fn: any, res: T) => () => T}
   * @internal
   */
  createEsmInitializer = (fn, res) => () => (fn && (res = fn((fn = 0))), res);
  /**
   * __commonJSMin
   *
   * @type {<T extends { exports: any }>(cb: any, mod: { exports: any }) => () => T}
   * @internal
   */
  createCjsInitializer = (cb, mod) => () => (
    mod || cb((mod = { exports: {} }).exports, mod), mod.exports
  );
  /** @internal */
  __toESM = __toESM;
  /** @internal */
  __toCommonJS = __toCommonJS;
  /** @internal */
  __exportAll = __exportAll;
  /**
   * @param {boolean} [isNodeMode]
   * @returns {(mod: any) => any}
   * @internal
   */
  __toDynamicImportESM = (isNodeMode) => (mod) => __toESM(mod.default, isNodeMode);
  /** @internal */
  __reExport = __reExport;

  cache = /** @type {string[]} */ ([]);
  timeout = /** @type {NodeJS.Timeout | null} */ (null);
  timeoutSetLength = 0;

  /** @type {(module: string) => void} */
  sendModuleRegisteredMessage = (() => {
    const self = this;

    /**
     * @param {string} module
     */
    return function sendModuleRegisteredMessage(module) {
      if (!self.messenger) {
        return;
      }
      self.cache.push(module);
      this.timeout = safetyInvokeWithSetTimeout(self.flush.bind(this));
    };
  })();

  flush() {
    if (this.cache.length > this.timeoutSetLength) {
      this.timeoutSetLength = this.cache.length;
      this.timeout = safetyInvokeWithSetTimeout(this.flush.bind(this));
      return;
    }

    this.messenger.send({
      type: 'hmr:module-registered',
      modules: this.cache,
    });
    this.cache.length = 0;
    this.timeoutSetLength = 0;
    this.timeout = null;
  }
}

/**
 * In lower React Native versions, `setTimeout` is cannot be used in `rolldown:hmr` initialization phase because `InitializeCore` of React Native is not evaluated yet.
 * 
 * `rolldown:hmr` -> `InitializeCore` -> Define polyfills (e.g, `setTimeout`)
 *
 * @param {() => void} fn 
 */
function safetyInvokeWithSetTimeout(fn) {
  if (typeof setTimeout === 'function') {
    return setTimeout(fn);
  }

  fn();

  return null;
}
