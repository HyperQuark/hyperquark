const { readFileSync } = require("fs");

require("node-dir").files(__dirname, function(err, files) {
  console.log(files.filter(a => !/(\.git(?!ignore)|node_modules|\.npm|\.cache|build|(?<!vite)\.config|\.bash|\.glitch|package-lock|shrinkwrap|pnpm)/.test(a)).reduce((a, b) => {
    return (readFileSync(b, {encoding:"utf-8"}).split("\n").length - 1) + a;
  }, 0));
});
