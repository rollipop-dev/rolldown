export { value as cjsValue } from './known.cjs';
export { default as cjsDefault, value as unknownValue } from './unknown.cjs';

globalThis.cjsReexportExecuted = true;
