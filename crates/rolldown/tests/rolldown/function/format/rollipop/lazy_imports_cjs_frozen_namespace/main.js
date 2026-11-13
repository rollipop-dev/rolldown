import * as clientNs from './client.cjs';
import { init } from './client.cjs';

// Freezing the shared namespace must not break importers evaluated after
// client.cjs gains a key.
Object.freeze(clientNs);
init();

globalThis.__rollipop_frozen_namespace_result = import('./reader.js').then((reader) => ({
  client: reader.readClient(),
  stillFrozen: Object.isFrozen(clientNs),
  frozenKeys: Object.keys(clientNs).sort(),
  readerKeys: Object.keys(reader.clientNs).sort(),
}));
