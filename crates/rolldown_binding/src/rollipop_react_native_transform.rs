//! NAPI surface for the standalone React Native SWC transform pipeline.
//!
//! Exposes a stateful class so that SWC `.wasm` plugins (the expensive
//! preload step) are compiled exactly once per [`BindingRollipopReactNativeTransformer`]
//! instance, then reused across every `transform` / `transformSync` call.
//! Mirrors the rolldown plugin's preload semantics for the NAPI path.

use std::sync::Arc;

use napi::{Task, bindgen_prelude::AsyncTask};
use napi_derive::napi;
use rollipop_react_native_transform::{TransformInput, Transformer};

use crate::options::plugin::BindingRollipopReactNativePluginConfig;

#[napi_derive::napi(object, object_from_js = false)]
#[derive(Debug)]
pub struct BindingRollipopReactNativeTransformResult {
  pub code: String,
  /// JSON-encoded source map (sourcemap V3). Parse on the JS side if you
  /// need a structured representation.
  pub map: String,
}

#[napi]
pub struct BindingRollipopReactNativeTransformer {
  inner: Arc<Transformer>,
}

#[napi]
impl BindingRollipopReactNativeTransformer {
  /// Construct a new transformer. SWC `.wasm` plugins listed in `config`
  /// are read from disk and compiled here exactly once — subsequent
  /// `transform` / `transformSync` calls reuse the compiled modules.
  #[napi(constructor)]
  pub fn new(config: BindingRollipopReactNativePluginConfig) -> napi::Result<Self> {
    let (env_name, options) =
      config.into_parts().map_err(|e| napi::Error::from_reason(e.to_string()))?;
    let transformer =
      Transformer::new(env_name, options).map_err(|e| napi::Error::from_reason(e.to_string()))?;
    Ok(Self { inner: Arc::new(transformer) })
  }

  /// Asynchronously transform `code`. The module kind is inferred from the
  /// `filename` extension (matches rolldown's default extension table).
  #[napi(ts_return_type = "Promise<BindingRollipopReactNativeTransformResult>")]
  pub fn transform(
    &self,
    filename: String,
    code: String,
  ) -> AsyncTask<BindingRollipopReactNativeTransformTask> {
    AsyncTask::new(BindingRollipopReactNativeTransformTask {
      transformer: Arc::clone(&self.inner),
      filename,
      code,
    })
  }

  /// Synchronously transform `code`. The module kind is inferred from the
  /// `filename` extension (matches rolldown's default extension table).
  #[napi]
  pub fn transform_sync(
    &self,
    filename: String,
    code: String,
  ) -> napi::Result<BindingRollipopReactNativeTransformResult> {
    run(&self.inner, &filename, &code)
  }
}

fn run(
  transformer: &Transformer,
  filename: &str,
  code: &str,
) -> napi::Result<BindingRollipopReactNativeTransformResult> {
  let output = transformer
    .transform(TransformInput { filename, code, module_kind: None })
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;
  Ok(BindingRollipopReactNativeTransformResult { code: output.code, map: output.map_json })
}

pub struct BindingRollipopReactNativeTransformTask {
  transformer: Arc<Transformer>,
  filename: String,
  code: String,
}

#[napi]
impl Task for BindingRollipopReactNativeTransformTask {
  type JsValue = BindingRollipopReactNativeTransformResult;
  type Output = BindingRollipopReactNativeTransformResult;

  fn compute(&mut self) -> napi::Result<Self::Output> {
    run(&self.transformer, &self.filename, &self.code)
  }

  fn resolve(&mut self, _env: napi::Env, output: Self::Output) -> napi::Result<Self::JsValue> {
    Ok(output)
  }
}
