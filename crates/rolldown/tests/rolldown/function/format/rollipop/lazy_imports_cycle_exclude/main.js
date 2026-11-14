// Eager evaluation enters the p <-> q cycle at p (import order). Lazily, reading
// `c` first would enter it at q, and p would then read q's `c` before q declared
// it (TDZ). Excluding p keeps the eager entry point.
import { pthing } from './p.js';
import { c } from './q.js';

globalThis.__rollipop_cycle_exclude_result = { c, pthing };
