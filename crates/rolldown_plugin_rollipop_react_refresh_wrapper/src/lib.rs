use std::{borrow::Cow, fmt::Write, sync::LazyLock};

use regex::Regex;
use rolldown_common::ModuleType;
use rolldown_plugin::{
  HookTransformOutput, HookTransformOutputMap, HookUsage, Plugin, SharedTransformPluginContext,
};
use rolldown_plugin_utils::to_string_literal;
use rolldown_utils::pattern_filter::{FilterResult, StringOrRegex, filter};

static REACT_COMP_RE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new("extends\\s+(?:React\\.)?(?:Pure)?Component").unwrap());
const REFRESH_CONTENT: &str = "$RefreshReg$(";

#[derive(Debug)]
pub struct RollipopReactRefreshWrapperPluginOptions {
  pub cwd: String,
  pub include: Vec<StringOrRegex>,
  pub exclude: Vec<StringOrRegex>,
  pub jsx_import_source: Option<String>,
}

#[derive(Debug)]
pub struct RollipopReactRefreshWrapperPlugin {
  cwd: String,
  include: Vec<StringOrRegex>,
  exclude: Vec<StringOrRegex>,
}

impl RollipopReactRefreshWrapperPlugin {
  pub fn new(options: RollipopReactRefreshWrapperPluginOptions) -> Self {
    Self { cwd: options.cwd, include: options.include, exclude: options.exclude }
  }

  fn add_refresh_wrapper(&self, code: &str, id: &str, module_type: &ModuleType) -> Option<String> {
    if !((/* is_jsx */is_jsx(id, module_type))
      || (/* has_refresh */memchr::memmem::find(code.as_bytes(), REFRESH_CONTENT.as_bytes()).is_some())
      || (/* only_react_comp */REACT_COMP_RE.is_match(code)))
    {
      return None;
    }

    let escaped_id = to_string_literal(id);
    let mut new_code = code.to_string();
    write!(
      new_code,
      "\
\nif (import.meta.hot) {{
  if (import.meta.hot.refresh == null) throw new Error('react-refresh runtime is not initialized');
  import.meta.hot.accept((nextExports) => {{
    if (!nextExports) return;
    if (import.meta.hot.refreshUtils.isReactRefreshBoundary(nextExports)) {{
      import.meta.hot.refreshUtils.enqueueUpdate();
    }}
  }});
}}
"
    )
    .unwrap();

    if (/* is_jsx */is_jsx(id, module_type))
      || (/* has_refresh */memchr::memmem::find(code.as_bytes(), REFRESH_CONTENT.as_bytes()).is_some())
    {
      write!(
        new_code,
        "\
function $RefreshReg$(type, id) {{ return __ReactRefresh.register(type, {escaped_id} + ' ' + id); }}
function $RefreshSig$() {{ return __ReactRefresh.createSignatureFunctionForTransform(); }}
",
      )
      .unwrap();
    }

    Some(new_code)
  }
}

impl Plugin for RollipopReactRefreshWrapperPlugin {
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed("builtin:rollipop-react-refresh-wrapper")
  }

  fn register_hook_usage(&self) -> HookUsage {
    HookUsage::Transform
  }

  async fn transform(
    &self,
    _ctx: SharedTransformPluginContext,
    args: &rolldown_plugin::HookTransformArgs<'_>,
  ) -> rolldown_plugin::HookTransformReturn {
    if matches!(
      filter(Some(&self.exclude), Some(&self.include), args.id, &self.cwd),
      FilterResult::Match(false) | FilterResult::NoneMatch(false)
    ) {
      return Ok(None);
    }

    let Some(code) = self.add_refresh_wrapper(args.code, args.id, args.module_type) else {
      return Ok(None);
    };
    Ok(Some(HookTransformOutput {
      code: Some(code),
      map: HookTransformOutputMap::Null,
      ..Default::default()
    }))
  }
}

fn is_jsx(id: &str, module_type: &ModuleType) -> bool {
  if matches!(module_type, ModuleType::Jsx | ModuleType::Tsx) {
    return true;
  }

  let id_without_query = id.split_once('?').map_or(id, |(id, _)| id);
  id_without_query.ends_with('x')
}
