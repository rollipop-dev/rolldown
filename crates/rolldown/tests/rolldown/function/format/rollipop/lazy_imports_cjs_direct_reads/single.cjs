let created = 0;
module.exports = {
  version: 1,
  get client() {
    if (!this._client) this._client = { id: ++created };
    return this._client;
  },
};
Object.defineProperty(module.exports, 'isExports', {
  get() {
    return this === module.exports;
  },
});
