import { defineTest } from 'rolldown-tests';
import { viteDynamicImportVarsPlugin, viteImportGlobPlugin } from '@rollipop/rolldown/experimental';

export default defineTest({
  config: {
    plugins: [viteDynamicImportVarsPlugin({}), viteImportGlobPlugin()],
  },
  async afterTest() {
    await import('./assert.mjs');
  },
});
