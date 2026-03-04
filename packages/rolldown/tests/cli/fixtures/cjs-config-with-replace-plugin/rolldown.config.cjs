const rolldown = require('@rollipop/rolldown');
const plugin = require('@rollipop/rolldown/plugins');

module.exports = rolldown.defineConfig({
  input: './index.js',
  plugins: [
    plugin.replacePlugin({
      __rolldown: '1',
    }),
  ],
});
