use derive_more::Debug;
use rolldown_plugin_rollipop_worklets::RollipopWorkletsPlugin;

#[napi_derive::napi(object, object_to_js = false)]
#[derive(Debug)]
pub struct BindingRollipopWorkletsPluginConfig {
  pub root: String,
  pub plugin_version: String,
  pub is_release: bool,
}

impl From<BindingRollipopWorkletsPluginConfig> for RollipopWorkletsPlugin {
  fn from(value: BindingRollipopWorkletsPluginConfig) -> Self {
    Self { root: value.root, plugin_version: value.plugin_version, is_release: value.is_release }
  }
}
