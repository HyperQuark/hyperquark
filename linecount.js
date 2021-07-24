require("node-dir").files(__dirname, function(err, files) {
  console.log(files.filter(a => !/(\.git(?!ignore)|node_modules|\.npm|\.cache|build|(?<!vite)\.config|\.bash|\.glitch|package-lock|shrinkwrap|pnpm)/.test(a)));
});
