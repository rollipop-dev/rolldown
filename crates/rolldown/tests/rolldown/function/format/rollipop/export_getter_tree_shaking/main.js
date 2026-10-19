import { increment, used } from './dep.js';
import { forwarded } from './reexport.js';
import './side_effect_only.js';

globalThis.effects.push('entry');
const values = [[used, forwarded]];
increment();
values.push([used, forwarded]);
increment();
values.push([used, forwarded]);
globalThis.result = {
  values,
  effects: globalThis.effects,
};
