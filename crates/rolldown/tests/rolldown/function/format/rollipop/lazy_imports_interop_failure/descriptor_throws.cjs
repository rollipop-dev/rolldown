// Interop fails after an earlier export descriptor has been processed.
module.exports = new Proxy(
  { first: 'first', second: 'second' },
  {
    getOwnPropertyDescriptor(target, key) {
      if (key === 'second') throw new Error('descriptor-second');
      return Reflect.getOwnPropertyDescriptor(target, key);
    },
  },
);
