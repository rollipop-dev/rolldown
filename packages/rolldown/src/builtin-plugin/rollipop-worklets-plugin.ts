import { BuiltinPlugin } from './utils';

import type { BindingRollipopWorkletsPluginConfig } from '../binding.cjs';

export function rollipopWorkletsPlugin(config: BindingRollipopWorkletsPluginConfig): BuiltinPlugin {
  return new BuiltinPlugin('builtin:rollipop-worklets', config);
}
