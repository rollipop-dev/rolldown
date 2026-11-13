// Reads the package's exports later, through its index.
import { registered, nested, i18n, t } from './lib/index.js';

export const later = () => [registered, nested, i18n, t('hello')];
