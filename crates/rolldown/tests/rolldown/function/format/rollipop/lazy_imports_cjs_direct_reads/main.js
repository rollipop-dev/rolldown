import './warm.js';
import { count, increment, addKey, addedLater } from './live.cjs';
import { value } from './hot.cjs';
import { boom } from './throws.cjs';
import { fromGetter } from './getter.cjs';
import esmoduleDefault from './esmodule.cjs';
import { version, client, isExports } from './single.cjs';
import { clientViaNamespace } from './single_user.js';
import { fromProxy } from './proxy.cjs';

const log = (globalThis.__rollipop_lazy_direct_log ||= []);
const result = {};
result.beforeAccess = log.slice();
result.countBefore = count;
increment();
result.countAfter = count;
result.addedBefore = addedLater;
addKey();
result.addedAfter = addedLater;
result.afterLive = log.slice();
result.hot = value;
result.fromGetter = fromGetter;
result.esmoduleDefault = esmoduleDefault;
const errors = [];
for (let i = 0; i < 2; i++) {
  try {
    errors.push(boom);
  } catch (error) {
    errors.push(error.message);
  }
}
result.errors = errors;
// Accessors see `module.exports` as `this` on every read.
result.version = version;
result.sameClient = client === clientViaNamespace() && client === client;
result.isExports = [isExports, isExports];
result.fromProxy = [fromProxy, fromProxy];
globalThis.__rollipop_lazy_direct_result = result;
