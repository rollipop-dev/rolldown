// Caches the Node-interop namespace of client.cjs before `init()` adds a key.
import * as clientNs from './client.cjs';
export const earlyNodeNs = clientNs;
