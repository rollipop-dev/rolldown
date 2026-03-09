use crate::types::binding_string_or_regex::{
  BindingStringOrRegex, bindingify_string_or_regex_array,
};
use rolldown_plugin_rollipop_react_refresh_wrapper::RollipopReactRefreshWrapperPluginOptions;

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug, Default)]
pub struct BindingRollipopReactRefreshWrapperPluginConfig {
  pub cwd: String,
  pub include: Option<Vec<BindingStringOrRegex>>,
  pub exclude: Option<Vec<BindingStringOrRegex>>,
  pub jsx_import_source: Option<String>,
}

impl From<BindingRollipopReactRefreshWrapperPluginConfig>
  for RollipopReactRefreshWrapperPluginOptions
{
  fn from(value: BindingRollipopReactRefreshWrapperPluginConfig) -> Self {
    Self {
      cwd: value.cwd,
      include: value.include.map(bindingify_string_or_regex_array).unwrap_or_default(),
      exclude: value.exclude.map(bindingify_string_or_regex_array).unwrap_or_default(),
      jsx_import_source: value.jsx_import_source,
    }
  }
}
