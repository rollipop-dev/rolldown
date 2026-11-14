import { value } from './dep.js';
import cjs, { named } from './cjs.cjs';

(globalThis.__rollipop_lazy_no_proxy_log ??= []).push('main');
globalThis.__rollipop_lazy_no_proxy_result = {
  value,
  cjs: cjs.named,
  named,
  log: globalThis.__rollipop_lazy_no_proxy_log.slice(),
};
