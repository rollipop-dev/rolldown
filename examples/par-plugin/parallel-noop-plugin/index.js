import path from 'node:path';
import { defineParallelPlugin } from '@rollipop/rolldown/experimental';

/** @type {import('@rollipop/rolldown').DefineParallelPluginResult<void>} */
export default defineParallelPlugin(path.resolve(import.meta.dirname, './impl.js'));
