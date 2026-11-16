// `default` reads need the interop namespace, so these bindings build it.
import flaggedDefault from './esmodule_flag_throws.cjs';
import descriptorDefault from './descriptor_throws.cjs';
import { namedOnly } from './named_only.js';

function readTwice(read) {
  const results = [];
  for (let i = 0; i < 2; i++) {
    try {
      results.push(`value:${read()}`);
    } catch (error) {
      results.push(`threw:${error.message}`);
    }
  }
  return results;
}

globalThis.__rollipop_interop_failure_result = {
  immediate: readTwice(() => flaggedDefault),
  afterDescriptor: readTwice(() => descriptorDefault.first),
  namedOnly: namedOnly(readTwice),
};
