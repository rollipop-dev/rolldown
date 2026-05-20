//! SWC `.wasm` plugin runtime via wasmer. Compiled only with the
//! `wasm_plugins` feature — see the crate-level Cargo manifest for the
//! target gating rationale.

use std::sync::Arc;

use swc_common::comments::SingleThreadedComments;
use swc_common::plugin::metadata::TransformPluginMetadataContext;
use swc_common::plugin::serialized::{PluginSerializedBytes, VersionedSerializable};
use swc_common::sync::Lrc;
use swc_common::{Mark, SourceMap};
use swc_ecma_ast::{Module, Program};
use swc_plugin_runner::create_plugin_transform_executor;
use swc_plugin_runner::plugin_module_bytes::{CompiledPluginModuleBytes, RawPluginModuleBytes};

use crate::SwcWasmPlugin;

struct Preloaded {
  compiled: CompiledPluginModuleBytes,
  config: Arc<serde_json::Value>,
}

pub struct WasmPlugins {
  runtime: Arc<dyn swc_plugin_runner::runtime::Runtime>,
  plugins: Vec<Preloaded>,
  env_name: String,
}

/// `SWC_ENV` → `NODE_ENV` → `"development"` — matches `@swc/core`.
fn default_env_name() -> String {
  std::env::var("SWC_ENV")
    .or_else(|_| std::env::var("NODE_ENV"))
    .unwrap_or_else(|_| "development".into())
}

impl WasmPlugins {
  pub fn new(plugins: Vec<SwcWasmPlugin>, env_name: Option<String>) -> Result<Self, anyhow::Error> {
    let runtime: Arc<dyn swc_plugin_runner::runtime::Runtime> =
      Arc::new(swc_plugin_backend_wasmer::WasmerRuntime);
    let plugins = plugins
      .into_iter()
      .map(|p| {
        let bytes = std::fs::read(&p.path)
          .map_err(|e| anyhow::anyhow!("Failed to read wasm plugin '{}': {e}", p.path))?;
        let raw = RawPluginModuleBytes::new(p.path, bytes);
        let compiled = CompiledPluginModuleBytes::from_raw_module(&*runtime, raw);
        Ok(Preloaded { compiled, config: Arc::new(p.config) })
      })
      .collect::<Result<Vec<_>, anyhow::Error>>()?;
    Ok(Self { runtime, plugins, env_name: env_name.unwrap_or_else(default_env_name) })
  }

  pub fn len(&self) -> usize {
    self.plugins.len()
  }

  pub fn run(
    &self,
    cm: &Lrc<SourceMap>,
    unresolved_mark: Mark,
    comments: &SingleThreadedComments,
    filename: &str,
    program: &mut Program,
  ) -> Result<(), anyhow::Error> {
    if self.plugins.is_empty() {
      return Ok(());
    }

    let owned = std::mem::replace(program, Program::Module(Module::default()));
    let initial = PluginSerializedBytes::try_serialize(&VersionedSerializable::new(owned))?;

    let final_bytes = swc_plugin_proxy::COMMENTS.set(
      &swc_plugin_proxy::HostCommentsStorage { inner: Some(comments.clone()) },
      || -> Result<PluginSerializedBytes, anyhow::Error> {
        let mut serialized = initial;
        for p in &self.plugins {
          let module = p.compiled.clone_module(&*self.runtime);
          let metadata = Arc::new(TransformPluginMetadataContext::new(
            Some(filename.to_string()),
            self.env_name.clone(),
            None,
          ));
          let mut executor = create_plugin_transform_executor(
            cm,
            &unresolved_mark,
            &metadata,
            None,
            Box::new(module),
            Some((*p.config).clone()),
            Arc::clone(&self.runtime),
          );
          serialized = executor.transform(&serialized, Some(true))?;
        }
        Ok(serialized)
      },
    )?;

    *program = final_bytes.deserialize()?.into_inner();
    Ok(())
  }
}
