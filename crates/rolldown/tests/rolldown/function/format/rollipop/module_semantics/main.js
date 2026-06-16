import defaultValue, { getMutableValue, incrementMutableValue, namedValue } from './exports.js';
import DefaultClass from './default_class.js';
import defaultExpression from './default_expr.js';
import defaultFunction from './default_fn.js';
import * as ns from './exports.js';
import {
  ClassExport,
  'dashed-name' as dashedName,
  destructuredValue,
  fnExport,
  importedThenExported,
  localAlias,
} from './local_exports.js';
import {
  bumpStarMutable,
  default as reexportedDefault,
  getStarMutable,
  namedValue as reexportedNamed,
  nsExport,
  'quoted-reexport' as quotedReexport,
  renamed,
  starMutable,
  starValue,
} from './reexport.js';
import { unusedValue } from './unused.js';

incrementMutableValue();
bumpStarMutable();

const classExport = new ClassExport();
const defaultClass = new DefaultClass();
const dynamicResult = import('./dynamic.js').then((dynamicNs) => ({
  dynamicDefault: dynamicNs.default,
  dynamicNamed: dynamicNs.dynamicNamed,
}));

globalThis.__rollipop_module_semantics_result = dynamicResult.then((dynamic) => ({
  defaultValue,
  namedValue,
  namespaceDefault: ns.default,
  namespaceNamed: ns.namedValue,
  namespaceLiveValue: ns.mutableValue,
  namespaceToStringTag: Object.prototype.toString.call(ns),
  liveValue: getMutableValue(),
  reexportedDefault,
  reexportedNamed,
  quotedReexport,
  renamed,
  namespaceExportDefault: nsExport.default,
  namespaceExportNamed: nsExport.namedValue,
  namespaceExportLiveValue: nsExport.mutableValue,
  localAlias,
  importedThenExported,
  dashedName,
  destructuredValue,
  fnExport: fnExport(),
  classExport: classExport.value(),
  starValue,
  starMutable,
  starLiveValue: getStarMutable(),
  defaultFunction: defaultFunction(),
  defaultClass: defaultClass.value(),
  defaultExpression,
  ...dynamic,
  reexportModuleLoaded: globalThis.__rollipop_reexport_module_loaded === true,
  sideEffectReexportLoaded: globalThis.__rollipop_side_effect_reexport_loaded === true,
}));
