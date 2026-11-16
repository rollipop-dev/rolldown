// `typeof` and `!` only read the member, so the namespace isn't needed.
import * as fnNs from './fn.cjs';
export const typeofHelper = () => [typeof fnNs.helper, !fnNs.helper];
