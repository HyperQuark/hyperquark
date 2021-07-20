const path = require("path");
const assemblyScript = require("@sebastianspeitel/rollup-plugin-assemblyscript-loader");

export default {
  css: {
    modules: {
      localsConvention: 'camelCaseOnly'
    }
  },
  plugins: [assemblyScript({
    include: /^.*?\.as$/
  })],
  build: {
    outDir: "build"
  },
  server: {
    strictPort: true,
    hmr: {
      port: 443 // Run the websocket server on the SSL port
    }
  },
 /* resolve: {
    alias: {
      find: '@/',
      replacement: path.resolve(__dirname, './src')
    }
  },*/
}