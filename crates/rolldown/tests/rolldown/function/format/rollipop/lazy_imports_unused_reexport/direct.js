import { sharedE } from './register_e.js';
import { sharedF } from './register_f.js';
import { sharedG } from './register_g.js';
import { sharedH } from './register_h.js';

// Never called, so these lazy imports alone never run the register modules.
export const readShared = () => [sharedE, sharedF, sharedG, sharedH];
