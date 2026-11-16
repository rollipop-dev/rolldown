// A Proxy that lists a key without describing it.
module.exports = new Proxy(
  {},
  {
    get(_, key) {
      return key === 'fromProxy' ? 'proxy-value' : undefined;
    },
    ownKeys() {
      return ['fromProxy'];
    },
  },
);
