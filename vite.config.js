const path = require("path");

export default {
  css: {
    modules: {
      localsConvention: 'camelCaseOnly'
    }
  },
  build: {
    outDir: "build"
  },
  server: {
    strictPort: true,
    hmr: {
      port: 443 // Run the websocket server on the SSL port
    }
  },
  resolve: {
    alias: {
      find: '@/',
      replacement: path.resolve(__dirname, './src')
    }
  },
}