// MARK: - Rollipop

use napi::Either;
use napi_derive::napi;
use oxc_transform_napi::TransformOptions as OxcTransformOptions;
use rolldown_common::{JsxOptions, ReactRefreshOptions};

use crate::types::binding_string_or_regex::{
  BindingStringOrRegex, bindingify_string_or_regex_array,
};

#[napi(object, object_to_js = false)]
#[derive(Default)]
pub struct BindingTransformOptions {
  pub options: OxcTransformOptions,
  // MARK: - Rollipop
  #[napi(ts_type = "'preserve' | 'react' | 'react-jsx' | 'disable' | BindingJsxOptions")]
  pub jsx: Option<Either<String, BindingJsxOptions>>,
}

// MARK: - Rollipop
/// Configure how TSX and JSX are transformed.
///
/// Mirrors Oxc's JSX options with Rollipop-only refresh filters.
#[napi(object, object_to_js = false)]
#[derive(Default)]
pub struct BindingJsxOptions {
  /// Decides which runtime to use.
  #[napi(ts_type = "'classic' | 'automatic'")]
  pub runtime: Option<String>,
  /// Emit development-specific information, such as `__source` and `__self`.
  pub development: Option<bool>,
  /// Toggles whether or not to throw an error if an XML namespaced tag name is used.
  pub throw_if_namespace: Option<bool>,
  /// Mark JSX elements and top-level React method calls as pure for tree shaking.
  pub pure: Option<bool>,
  /// Replaces the import source when importing functions.
  pub import_source: Option<String>,
  /// Replace the function used when compiling JSX expressions.
  pub pragma: Option<String>,
  /// Replace the component used when compiling JSX fragments.
  pub pragma_frag: Option<String>,
  /// Enable React Fast Refresh.
  pub refresh: Option<Either<bool, BindingReactRefreshOptions>>,
}

// MARK: - Rollipop
/// React Fast Refresh options.
#[napi(object, object_to_js = false)]
#[derive(Default)]
pub struct BindingReactRefreshOptions {
  /// File patterns to transform. Empty means all files that enter the transform pipeline.
  pub include: Option<Vec<BindingStringOrRegex>>,
  /// File patterns to skip.
  pub exclude: Option<Vec<BindingStringOrRegex>>,
  /// Specify the identifier of the refresh registration variable.
  pub refresh_reg: Option<String>,
  /// Specify the identifier of the refresh signature variable.
  pub refresh_sig: Option<String>,
  pub emit_full_signatures: Option<bool>,
}

impl From<BindingReactRefreshOptions> for ReactRefreshOptions {
  fn from(value: BindingReactRefreshOptions) -> Self {
    Self {
      include: value.include.map(bindingify_string_or_regex_array).unwrap_or_default(),
      exclude: value.exclude.map(bindingify_string_or_regex_array).unwrap_or_default(),
      refresh_reg: value.refresh_reg,
      refresh_sig: value.refresh_sig,
      emit_full_signatures: value.emit_full_signatures,
    }
  }
}

impl From<BindingJsxOptions> for JsxOptions {
  fn from(value: BindingJsxOptions) -> Self {
    Self {
      runtime: value.runtime,
      development: value.development,
      throw_if_namespace: value.throw_if_namespace,
      pure: value.pure,
      import_source: value.import_source,
      pragma: value.pragma,
      pragma_frag: value.pragma_frag,
      refresh: value.refresh.map(|refresh| match refresh {
        Either::A(enabled) => itertools::Either::Left(enabled),
        Either::B(options) => itertools::Either::Right(options.into()),
      }),
    }
  }
}
