import { registry } from './registry.js';
import { pluginName } from './lib/index.js';

// Reads the registry before reading any binding of the package.
const beforeRead = registry.get('plugin');
globalThis.__rollipop_exclude_barrel_result = { beforeRead, pluginName };
