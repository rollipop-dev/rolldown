import * as clientNs from './client.cjs';
import { init } from './client.cjs';
import { original } from './patched.cjs';
import { earlyNodeNs } from './node_early.mjs';
import './patcher.cjs';

// Adds `client` after client.cjs finished loading and after its namespaces were cached.
init();

// Importers evaluated after the new keys exist must see them, as each one did
// when it built its own `.t()` namespace.
globalThis.__rollipop_late_keys_result = Promise.all([
  import('./reader.js'),
  import('./node_reader.mjs'),
]).then(([reader, nodeReader]) => ({
  client: reader.readClient(),
  added: reader.readAdded(),
  original,
  nodeClient: nodeReader.readClient(),
  // The cached namespace is extended in place, so importers still share it.
  sameNamespace: reader.clientNs === clientNs,
  clientKeys: Object.keys(clientNs).sort(),
  earlyNodeKeys: Object.keys(earlyNodeNs).sort(),
}));
