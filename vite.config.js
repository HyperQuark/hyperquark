const path = require("path");
const { assemblyScript } = require("./asloader.js");

export default {
  css: {
    modules: {
      localsConvention: 'camelCaseOnly'
    }
  },
  plugins: [/*(assemblyScript()*/],
  build: {
    outDir: "build"
  },
  server: {
    strictPort: true,
    hmr: {
      port: 443 // Run the websocket server on the SSL port
    }
  }
}