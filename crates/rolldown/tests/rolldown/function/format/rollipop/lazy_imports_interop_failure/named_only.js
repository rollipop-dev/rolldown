// Named-only reads use `module.exports` directly and never run interop.
import { flagged } from './esmodule_flag_throws.cjs';
import { first } from './descriptor_throws.cjs';

export const namedOnly = (readTwice) => [...readTwice(() => flagged), ...readTwice(() => first)];
