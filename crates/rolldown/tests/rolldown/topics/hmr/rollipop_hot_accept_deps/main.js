import { message } from './dep.js';

globalThis.__rollipop_hmr_message = message;

import.meta.hot.accept(`./dep.js`, () => {});
import.meta.hot.accept(['./dep.js', `./dep.js`], () => {});
