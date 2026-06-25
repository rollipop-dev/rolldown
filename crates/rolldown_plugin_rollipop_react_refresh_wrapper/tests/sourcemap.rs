use std::{
  future::Future,
  pin::pin,
  sync::Arc,
  task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use arcstr::ArcStr;
use rolldown_common::{ModuleIdx, ModuleType, SourcemapChainElement};
use rolldown_plugin::{
  HookTransformArgs, HookTransformOutputMap, Plugin, PluginContext, PluginIdx,
  TransformPluginContext,
};
use rolldown_plugin_rollipop_react_refresh_wrapper::{
  RollipopReactRefreshWrapperPlugin, RollipopReactRefreshWrapperPluginOptions,
};
use rolldown_utils::{pattern_filter::StringOrRegex, unique_arc::UniqueArc};

fn block_on<F: Future>(future: F) -> F::Output {
  fn noop_raw_waker() -> RawWaker {
    fn clone(_: *const ()) -> RawWaker {
      noop_raw_waker()
    }
    fn wake(_: *const ()) {}
    fn wake_by_ref(_: *const ()) {}
    fn drop(_: *const ()) {}

    RawWaker::new(std::ptr::null(), &RawWakerVTable::new(clone, wake, wake_by_ref, drop))
  }

  // SAFETY: The raw waker never dereferences the data pointer and is only used
  // to poll plugin futures that complete synchronously in this test.
  let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
  let mut cx = Context::from_waker(&waker);
  let mut future = pin!(future);

  loop {
    match future.as_mut().poll(&mut cx) {
      Poll::Ready(output) => return output,
      Poll::Pending => std::thread::yield_now(),
    }
  }
}

#[test]
fn adds_refresh_wrapper_without_transforming_jsx_or_returning_map() {
  let input = ArcStr::from(
    "\
import { View } from 'react-native';

function HomeScreen() {
  const navigation = useNavigation();
  throw new Error('ROLLIPOP_SYMBOLICATE_RUNTIME_CHECK');
  return <View navigation={navigation} />;
}

export default HomeScreen;
",
  );
  let id = "/fixture/App.tsx";
  let plugin = RollipopReactRefreshWrapperPlugin::new(RollipopReactRefreshWrapperPluginOptions {
    cwd: "/fixture".to_string(),
    include: vec![StringOrRegex::String("**/*.tsx".to_string())],
    exclude: vec![],
    jsx_import_source: None,
  });
  let sourcemap_chain = UniqueArc::<Vec<SourcemapChainElement>>::new(vec![]);
  let ctx = Arc::new(TransformPluginContext::new(
    PluginContext::new_napi_context(),
    sourcemap_chain.weak_ref(),
    input.clone(),
    ArcStr::from(id),
    ModuleIdx::new(0),
    PluginIdx::new(0),
    None,
  ));
  let module_type = ModuleType::Tsx;
  let args = HookTransformArgs { id, code: &input, module_type: &module_type };

  let output = block_on(plugin.transform(ctx, &args)).unwrap().expect("plugin should transform");
  let code = output.code.expect("plugin should return code");
  assert!(code.contains("function $RefreshReg$(type, id)"));
  assert!(code.contains("import.meta.hot.accept"));
  assert!(code.contains("return <View navigation={navigation} />;"));
  assert!(!code.contains("jsx-dev-runtime"));
  assert!(matches!(output.map, HookTransformOutputMap::Null));
}

#[test]
fn adds_refresh_wrapper_to_js_file_when_module_type_is_jsx() {
  let input = ArcStr::from(
    "\
// @flow
import { View } from 'react-native';

function HomeScreen(props) {
  return <View title={props.title} />;
}

export default HomeScreen;
",
  );
  let id = "/fixture/FlowComponent.js";
  let plugin = RollipopReactRefreshWrapperPlugin::new(RollipopReactRefreshWrapperPluginOptions {
    cwd: "/fixture".to_string(),
    include: vec![StringOrRegex::String("**/*.js".to_string())],
    exclude: vec![],
    jsx_import_source: None,
  });
  let sourcemap_chain = UniqueArc::<Vec<SourcemapChainElement>>::new(vec![]);
  let ctx = Arc::new(TransformPluginContext::new(
    PluginContext::new_napi_context(),
    sourcemap_chain.weak_ref(),
    input.clone(),
    ArcStr::from(id),
    ModuleIdx::new(0),
    PluginIdx::new(0),
    None,
  ));
  let module_type = ModuleType::Jsx;
  let args = HookTransformArgs { id, code: &input, module_type: &module_type };

  let output = block_on(plugin.transform(ctx, &args)).unwrap().expect("plugin should transform");
  let code = output.code.expect("plugin should return code");
  assert!(code.contains("function $RefreshReg$(type, id)"));
  assert!(code.contains("import.meta.hot.accept"));
  assert!(matches!(output.map, HookTransformOutputMap::Null));
}

#[test]
fn adds_refresh_wrapper_to_js_with_refresh_content() {
  let input = ArcStr::from(
    "\
function HomeScreen() {
  return React.createElement(View);
}
var _c;
$RefreshReg$(_c, 'HomeScreen');
",
  );
  let id = "/fixture/App.js";
  let plugin = RollipopReactRefreshWrapperPlugin::new(RollipopReactRefreshWrapperPluginOptions {
    cwd: "/fixture".to_string(),
    include: vec![StringOrRegex::String("**/*.js".to_string())],
    exclude: vec![],
    jsx_import_source: None,
  });
  let sourcemap_chain = UniqueArc::<Vec<SourcemapChainElement>>::new(vec![]);
  let ctx = Arc::new(TransformPluginContext::new(
    PluginContext::new_napi_context(),
    sourcemap_chain.weak_ref(),
    input.clone(),
    ArcStr::from(id),
    ModuleIdx::new(0),
    PluginIdx::new(0),
    None,
  ));
  let module_type = ModuleType::Js;
  let args = HookTransformArgs { id, code: &input, module_type: &module_type };

  let output = block_on(plugin.transform(ctx, &args)).unwrap().expect("plugin should transform");
  let code = output.code.expect("plugin should return code");
  assert!(code.contains("function $RefreshReg$(type, id)"));
  assert!(code.contains("import.meta.hot.accept"));
  assert!(matches!(output.map, HookTransformOutputMap::Null));
}

#[test]
fn adds_refresh_boundary_to_js_react_class_without_refresh_helpers() {
  let input = ArcStr::from(
    "\
class HomeScreen extends React.Component {
  render() {
    return React.createElement(View);
  }
}
",
  );
  let id = "/fixture/App.js";
  let plugin = RollipopReactRefreshWrapperPlugin::new(RollipopReactRefreshWrapperPluginOptions {
    cwd: "/fixture".to_string(),
    include: vec![StringOrRegex::String("**/*.js".to_string())],
    exclude: vec![],
    jsx_import_source: None,
  });
  let sourcemap_chain = UniqueArc::<Vec<SourcemapChainElement>>::new(vec![]);
  let ctx = Arc::new(TransformPluginContext::new(
    PluginContext::new_napi_context(),
    sourcemap_chain.weak_ref(),
    input.clone(),
    ArcStr::from(id),
    ModuleIdx::new(0),
    PluginIdx::new(0),
    None,
  ));
  let module_type = ModuleType::Js;
  let args = HookTransformArgs { id, code: &input, module_type: &module_type };

  let output = block_on(plugin.transform(ctx, &args)).unwrap().expect("plugin should transform");
  let code = output.code.expect("plugin should return code");
  assert!(code.contains("import.meta.hot.accept"));
  assert!(!code.contains("function $RefreshReg$(type, id)"));
  assert!(matches!(output.map, HookTransformOutputMap::Null));
}

#[test]
fn skips_modules_without_refresh_content() {
  let input = ArcStr::from("export const runtime = true;\n");
  let id = "\0rolldown/runtime.js";
  let plugin = RollipopReactRefreshWrapperPlugin::new(RollipopReactRefreshWrapperPluginOptions {
    cwd: "/fixture".to_string(),
    include: vec![StringOrRegex::String("**/*.js".to_string())],
    exclude: vec![],
    jsx_import_source: None,
  });
  let sourcemap_chain = UniqueArc::<Vec<SourcemapChainElement>>::new(vec![]);
  let ctx = Arc::new(TransformPluginContext::new(
    PluginContext::new_napi_context(),
    sourcemap_chain.weak_ref(),
    input.clone(),
    ArcStr::from(id),
    ModuleIdx::new(0),
    PluginIdx::new(0),
    None,
  ));
  let module_type = ModuleType::Js;
  let args = HookTransformArgs { id, code: &input, module_type: &module_type };

  let output = block_on(plugin.transform(ctx, &args)).unwrap();
  assert!(output.is_none());
}
