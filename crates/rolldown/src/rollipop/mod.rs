mod constants;
pub mod export_reads;
mod hmr;
mod module_ids;

pub use constants::HMR_RUNTIME_NAME;
pub use hmr::wrap_hmr_patch;
pub use module_ids::RollipopModuleIds;

pub const ROLLIPOP_RUNTIME: &str = include_str!("../runtime/rollipop-runtime.js");
// See internal-docs/rollipop-lazy-imports/implementation.md
pub const ROLLIPOP_LAZY_RUNTIME: &str = include_str!("../runtime/rollipop-runtime-lazy.js");

/// Whether import declarations should use lazy holders (`__rollipop_lazy__`).
pub fn is_lazy_imports_enabled(options: &rolldown_common::NormalizedBundlerOptions) -> bool {
  options.lazy_imports.is_some()
    && matches!(options.format, rolldown_common::OutputFormat::Rollipop)
    && !options.is_dev_mode_enabled()
}

pub const ROLLIPOP_GLOBAL_NAME: &str = "global";
pub const ROLLIPOP_MODULE_NAME: &str = "module";
pub const ROLLIPOP_EXPORTS_NAME: &str = "__rollipop_exports__";
pub const ROLLIPOP_REQUIRE_NAME: &str = "__rollipop_require__";
pub const ROLLIPOP_MODULES_NAME: &str = "__rollipop_modules__";
/// The fifth factory parameter in lazy-imports mode: `__rollipop_require__.l`.
/// Passing it as a parameter keeps a lazy import declaration (`l(5)`) the same
/// Hermes call as an eager one (`r(5)`), without a per-import property load.
pub const ROLLIPOP_LAZY_NAME: &str = "__rollipop_lazy__";

const ROLLIPOP_FACTORY_CALL: &str =
  "factory.call(module.exports, global, module, module.exports, __rollipop_require__);";
const ROLLIPOP_LAZY_FACTORY_CALL: &str = "factory.call(module.exports, global, module, module.exports, __rollipop_require__, __rollipop_require__.l);";

/// The base runtime, passing the lazy loader to factories when lazy imports are enabled.
pub fn rollipop_runtime(lazy_imports: bool) -> std::borrow::Cow<'static, str> {
  if lazy_imports {
    debug_assert!(ROLLIPOP_RUNTIME.contains(ROLLIPOP_FACTORY_CALL));
    ROLLIPOP_RUNTIME.replacen(ROLLIPOP_FACTORY_CALL, ROLLIPOP_LAZY_FACTORY_CALL, 1).into()
  } else {
    ROLLIPOP_RUNTIME.into()
  }
}
