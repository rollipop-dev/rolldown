import type { BindingRollipopReactRefreshWrapperPluginConfig } from '../binding.cjs';
import { normalizedStringOrRegex } from '../utils/normalize-string-or-regex';
import { BuiltinPlugin } from './utils';

export function rollipopReactRefreshWrapperPlugin(
  config: BindingRollipopReactRefreshWrapperPluginConfig,
): BuiltinPlugin {
  return new BuiltinPlugin('builtin:rollipop-react-refresh-wrapper', {
    cwd: config.cwd,
    include: normalizedStringOrRegex(config.include),
    exclude: normalizedStringOrRegex(config.exclude),
    jsxImportSource: config.jsxImportSource,
  });
}

export type { BindingRollipopReactRefreshWrapperPluginConfig as RollipopReactRefreshWrapperPluginConfig };
