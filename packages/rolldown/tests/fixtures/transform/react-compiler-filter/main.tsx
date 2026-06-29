import { DepCounter } from './node_modules/pkg/index';

export function Counter({ count }: { count: number }) {
  return <Text>{count}</Text>;
}

export { DepCounter };
