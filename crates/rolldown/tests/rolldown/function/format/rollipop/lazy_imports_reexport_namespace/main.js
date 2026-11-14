import { log } from './log.js';
import * as nsBarrel from './ns_barrel.js';
import * as starIndexNs from './star_index_ns.js';
import { starUsed, starLate } from './star_index.js';

log('main');
// Only `used` is read from each namespace, so the re-exported modules are kept
// for their side effects and run with their barrels.
const values = [nsBarrel.used, starIndexNs.starNsUsed, starUsed];
const required = require('./require_barrel.js');
values.push(required.requireUsed);
log('before late');
// Reads through a namespace member and through `export *` stay lazy.
values.push(nsBarrel.nsLate, starLate);

globalThis.__rollipop_reexport_namespace_result = import('./dynamic_barrel.js').then((mod) => ({
  values: [...values, mod.dynamicUsed],
  final: globalThis.__rollipop_reexport_namespace_log.slice(),
}));
