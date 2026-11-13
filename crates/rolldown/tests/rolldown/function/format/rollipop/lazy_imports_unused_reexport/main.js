import { log } from './log.js';
import { used } from './barrel.js';
import { used2 } from './barrel_import_export.js';
import { chained } from './outer.js';
import { usedCjs } from './barrel_cjs.js';
import { late, lateMarker } from './late_barrel.js';
import { usedE } from './barrel_shared.js';
import { chainedG } from './outer_g.js';
import { namerValue } from './namer.js';
import { deep, chainMarker } from './chain_outer.js';
import { readShared } from './direct.js';

log('main');
const values = [
  used,
  used2,
  chained,
  usedCjs,
  lateMarker,
  usedE,
  chainedG,
  namerValue,
  chainMarker,
  typeof readShared,
];
log('before late');
values.push(late, deep);
globalThis.__rollipop_unused_reexport_result = {
  values,
  final: globalThis.__rollipop_unused_reexport_log.slice(),
};
