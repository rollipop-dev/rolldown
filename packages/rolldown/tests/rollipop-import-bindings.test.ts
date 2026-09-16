import { runInNewContext } from 'node:vm';
import { rolldown } from '@rollipop/rolldown';
import { expect, test } from 'vitest';

test.each([false, true])(
  'keeps same-named modules distinct through a shared barrel: treeshake=%s',
  async (treeshake) => {
    const sources: Record<string, string> = {
      'entry.js': `
        import { encrypt, decrypt } from 'barrel.js';
        import { encode, decode } from 'reexport.js';
        globalThis.result = [encrypt(), decrypt(), encode(), decode()];
      `,
      'barrel.js': `
        export { encrypt } from 'encrypt/index.native.js';
        export { decrypt } from 'decrypt/index.native.js';
      `,
      'reexport.js': `
        export { encrypt as encode, decrypt as decode } from 'barrel.js';
        globalThis.keepReexport = true;
      `,
      'encrypt/index.native.js': `export function encrypt() { return 'encrypted'; }`,
      'decrypt/index.native.js': `export function decrypt() { return 'decrypted'; }`,
    };
    const bundle = await rolldown({
      input: 'entry.js',
      treeshake,
      plugins: [
        {
          name: 'fixture',
          resolveId: (id) => id,
          load: (id) => sources[id],
        },
      ],
    });
    try {
      const { output } = await bundle.generate({ format: 'rollipop' });
      const context = { result: undefined };
      runInNewContext(output[0].code, context);
      expect(context.result).toEqual(['encrypted', 'decrypted', 'encrypted', 'decrypted']);
    } finally {
      await bundle.close();
    }
  },
);
