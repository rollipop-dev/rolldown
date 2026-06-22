import { a } from './barrel/index.js';
import * as ns from './barrel/index.js';

export { a as exportedA } from './barrel/index.js';

globalThis.__rollipop_lazy_barrel_result = a;
globalThis.__rollipop_lazy_barrel_namespace_result = ns.a;
