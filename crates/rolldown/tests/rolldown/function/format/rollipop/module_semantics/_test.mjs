import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('./dist/main.js', import.meta.url), 'utf8');

assert.doesNotMatch(code, /\/\/#region unused\.js|should-be-tree-shaken/);

await import('./dist/main.js');

assert.deepEqual(await globalThis.__rollipop_module_semantics_result, {
  defaultValue: 'default-value',
  namedValue: 'named-value',
  namespaceDefault: 'default-value',
  namespaceNamed: 'named-value',
  namespaceLiveValue: 1,
  namespaceToStringTag: '[object Module]',
  liveValue: 1,
  reexportedDefault: 'default-value',
  reexportedNamed: 'named-value',
  quotedReexport: 'named-value',
  renamed: 'named-value',
  namespaceExportDefault: 'default-value',
  namespaceExportNamed: 'named-value',
  namespaceExportLiveValue: 1,
  localAlias: 'local-value',
  importedThenExported: 'named-value',
  dashedName: 'destructured-value',
  destructuredValue: 'destructured-value',
  fnExport: 'fn-export',
  classExport: 'class-export',
  starValue: 'star-value',
  starMutable: 1,
  starLiveValue: 1,
  defaultFunction: 'shadowed-function-default-name',
  defaultClass: 'shadowed-class-default-name',
  defaultExpression: 'shadowed-expression-default-name:expr',
  dynamicDefault: 'dynamic-default',
  dynamicNamed: 'dynamic-named',
  reexportModuleLoaded: true,
  sideEffectReexportLoaded: true,
});
