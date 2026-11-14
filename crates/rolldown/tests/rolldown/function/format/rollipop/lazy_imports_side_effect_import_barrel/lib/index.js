// A package index like `react-native-gesture-handler`: re-exports only.
export { registered } from './register.js';
export { nested } from './sub.js';
// The i18n shape: re-exported and read locally inside a helper.
import i18n from './i18n.js';
export { i18n };
export const t = (key) => i18n.t(key);
