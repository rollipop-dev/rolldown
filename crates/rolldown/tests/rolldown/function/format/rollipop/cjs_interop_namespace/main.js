import * as a from './a.js';
import * as b from './b.js';
import * as nodeMode from './node_mode.mjs';
import reassigned, { extra } from './reassigned.cjs';
import * as cycle from './cycle.cjs';

globalThis.__rollipop_cjs_interop_result = {
  sameNamespace: a.plain === b.plain,
  dynamicSameNamespace: import('./plain.cjs').then((dynamicPlain) => dynamicPlain === a.plain),
  plainDefault: a.plainDefault,
  named: a.named,
  plainKeys: Object.keys(a.plain).sort(),
  esmoduleDefault: b.esmoduleDefault,
  esmoduleNamed: b.esmodule.named,
  nodeDefaultIsExports:
    nodeMode.esmoduleDefault === b.esmodule.default
      ? 'esm-default'
      : typeof nodeMode.esmoduleDefault,
  nodeDistinctFromBabel: nodeMode.esmodule !== b.esmodule,
  reassigned: reassigned(),
  extra,
  cycleKeysDuringLoad: globalThis.__rollipop_cjs_cycle_keys_during_load,
  cycleKeys: Object.keys(cycle).sort(),
};
