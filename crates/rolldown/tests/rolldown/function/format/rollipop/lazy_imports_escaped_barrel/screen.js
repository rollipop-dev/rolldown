// Reads every package's re-export through its index, later.
import { req } from './req/index.js';
import { dyn } from './dyn/index.js';
import { esc } from './esc/index.js';
import { starns } from './starns/index.js';

export const later = () => [req, dyn, esc, starns];
