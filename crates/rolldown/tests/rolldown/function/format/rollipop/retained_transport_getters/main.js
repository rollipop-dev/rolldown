import { direct, local, nested } from './kept.js';
import { cjsDefault, cjsValue, unknownValue } from './cjs_reexport.js';

globalThis.result = [
  direct.value(),
  nested.group.value(),
  local,
  cjsValue,
  cjsDefault.value,
  unknownValue,
  globalThis.innerExecuted,
  globalThis.middleExecuted,
  globalThis.cjsReexportExecuted,
];
