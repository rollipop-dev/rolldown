import * as React from 'react';

export function Counter({ count }) {
  'use memo';
  return React.createElement(Text, null, count);
}
