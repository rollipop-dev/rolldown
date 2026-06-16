export default 'star-default';

export const starValue = 'star-value';

export let starMutable = 0;

export function bumpStarMutable() {
  starMutable += 1;
}

export function getStarMutable() {
  return starMutable;
}
