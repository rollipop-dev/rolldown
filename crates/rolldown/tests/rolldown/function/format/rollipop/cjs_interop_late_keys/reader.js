import * as clientNs from './client.cjs';
import { client } from './client.cjs';
import { added } from './patched.cjs';
export const readClient = () => client;
export const readAdded = () => added;
export { clientNs };
