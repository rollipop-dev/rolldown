import { increment, value } from './dep.js';
import { forwarded } from './reexport.js';

globalThis.effects.push('entry');
increment();
globalThis.values = [value, forwarded];
