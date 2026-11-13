import { flagged } from './esmodule_flag_throws.cjs';
import { first } from './descriptor_throws.cjs';

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
  immediate: readTwice(() => flagged),
  afterDescriptor: readTwice(() => first),
};
