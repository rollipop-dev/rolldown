// `ns['default']` of a plain CommonJS module is `module.exports` itself.
import * as plainNs from './plain.cjs';
export const computedDefault = () => plainNs['default'].plain;
