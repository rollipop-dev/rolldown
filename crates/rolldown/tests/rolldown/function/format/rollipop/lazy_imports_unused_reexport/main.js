import { log } from './log.js';
import { used } from './barrel.js';
import { used2 } from './barrel_import_export.js';
import { chained } from './outer.js';
import { usedCjs } from './barrel_cjs.js';
import { late, lateMarker } from './late_barrel.js';

log('main');
const values = [used, used2, chained, usedCjs, lateMarker];
log('before late');
values.push(late);
globalThis.__rollipop_unused_reexport_result = {
  values,
  final: globalThis.__rollipop_unused_reexport_log.slice(),
};
