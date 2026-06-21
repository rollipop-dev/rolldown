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

fn line_col(code: &str, needle: &str) -> (u32, u32) {
  let position = code.find(needle).expect("needle should exist in code");
  let mut line = 0;
  let mut line_start = 0;

  for (index, byte) in code.bytes().enumerate() {
    if index == position {
      break;
    }
    if byte == b'\n' {
      line += 1;
      line_start = index + 1;
    }
  }

  (line, u32::try_from(position - line_start).expect("fixture line should fit in u32"))
}

#[test]
fn preserves_throw_statement_location_after_refresh_wrapper() {
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
  assert!(code.contains("globalThis.$RefreshReg$"));
  let map = match output.map {
    HookTransformOutputMap::Sourcemap(map) => map,
    HookTransformOutputMap::Omitted | HookTransformOutputMap::Null => {
      panic!("plugin should return a sourcemap")
    }
  };
  let lookup_table = map.generate_lookup_table();
  let generated = line_col(&code, "ROLLIPOP_SYMBOLICATE_RUNTIME_CHECK");
  let original = map
    .lookup_source_view_token(&lookup_table, generated.0, generated.1)
    .expect("generated throw should map back to original source");

  assert_eq!(original.get_source_id(), Some(0));
  assert_eq!(original.get_src_line(), line_col(&input, "ROLLIPOP_SYMBOLICATE_RUNTIME_CHECK").0);
}
