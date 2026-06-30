import type {
  BindingStringOrRegex,
  OxcReactCompilerOptions,
  TransformOptions as OxcTransformOptions,
} from '../binding.cjs';
import type { InputOptions } from '../options/input-options';
import { normalizedStringOrRegex } from './normalize-string-or-regex';

type RolldownOxcTransformOptions = Omit<
  OxcTransformOptions,
  'sourceType' | 'lang' | 'cwd' | 'sourcemap' | 'define' | 'inject'
>;

type RolldownTransformOptions = {
  options: RolldownOxcTransformOptions;
  // MARK: - Rollipop
  reactCompiler?: OxcReactCompilerOptions;
  // MARK: - Rollipop
  jsxRefreshInclude?: BindingStringOrRegex[];
  jsxRefreshExclude?: BindingStringOrRegex[];
};

export interface NormalizedTransformOptions {
  define: Array<[string, string]> | undefined;
  inject: Record<string, string | [string, string]> | undefined;
  dropLabels: string[] | undefined;
  oxcTransformOptions: RolldownTransformOptions | undefined;
}

/**
 * Normalizes transform options by extracting `define`, `inject`, and `dropLabels` separately from OXC transform options.
 *
 * Prioritizes values from `transform.define`, `transform.inject`, and `transform.dropLabels` over deprecated top-level options.
 */
export function normalizeTransformOptions(inputOptions: InputOptions): NormalizedTransformOptions {
  const transform = inputOptions.transform;

  const define = transform?.define ? Object.entries(transform.define) : undefined;
  const inject = transform?.inject;
  const dropLabels = transform?.dropLabels;

  // Extract OXC transform options (excluding define, inject, and dropLabels)
  let oxcTransformOptions: RolldownTransformOptions | undefined;
  if (transform) {
    const {
      define: _define,
      inject: _inject,
      dropLabels: _dropLabels,
      reactCompiler,
      ...rest
    } = transform;
    // Only set oxcTransformOptions if there are actual options
    if (Object.keys(rest).length > 0 || reactCompiler != null) {
      if (rest.jsx === false) {
        rest.jsx = 'disable' as any;
      }
      // MARK: - Rollipop
      const jsxRefreshFilters = normalizeJsxRefreshFilters(rest);
      oxcTransformOptions = {
        options: rest as RolldownOxcTransformOptions,
        ...(reactCompiler != null ? { reactCompiler } : {}),
        ...jsxRefreshFilters,
      };
    }
  }

  return {
    define,
    inject,
    dropLabels,
    oxcTransformOptions,
  };
}

// MARK: - Rollipop
function normalizeJsxRefreshFilters(transformOptions: {
  jsx?: unknown;
}): Pick<RolldownTransformOptions, 'jsxRefreshInclude' | 'jsxRefreshExclude'> {
  const jsx = transformOptions.jsx;
  if (jsx == null || typeof jsx !== 'object') {
    return {};
  }

  const refresh = (jsx as { refresh?: unknown }).refresh;
  if (refresh == null || typeof refresh !== 'object') {
    return {};
  }

  const { include, exclude, ...oxcRefreshOptions } = refresh as {
    include?: BindingStringOrRegex | BindingStringOrRegex[];
    exclude?: BindingStringOrRegex | BindingStringOrRegex[];
  };

  transformOptions.jsx = {
    ...(jsx as Record<string, unknown>),
    refresh: oxcRefreshOptions,
  };

  return {
    jsxRefreshInclude: normalizedStringOrRegex<BindingStringOrRegex[]>(include),
    jsxRefreshExclude: normalizedStringOrRegex<BindingStringOrRegex[]>(exclude),
  };
}
