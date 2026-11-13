import { registry } from './registry.js';
import { pluginName } from './setup/register.js';
import { lazyValue } from './lazy.js';

// Reads the registry before reading any binding of the excluded module.
const beforeRead = registry.get('plugin');
const lazyLoadedEarly = (globalThis.__rollipop_lazy_exclude_log ?? []).includes('lazy');
globalThis.__rollipop_lazy_exclude_result = {
  beforeRead,
  lazyLoadedEarly,
  pluginName,
  lazyValue,
};
