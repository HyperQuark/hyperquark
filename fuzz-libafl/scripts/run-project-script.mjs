#!/usr/bin/env node

import { run } from "../../playground/dist/run-project.mjs";
import { argv } from "node:process";

console.log(argv);

let opts = {};

let i = 4;

let this_opt = null;
while (i < argv.length) {
  const arg = argv[i];
  if (/^--[\w_]+$/.test(arg)) {
    if (this_opt === null) {
      this_opt = arg.substring(2);
    } else {
      throw new Error(`Unexpected argument ${arg}`);
    }
  } else {
    if (this_opt !== null) {
      if (this_opt === "unroll_loops") {
        opts[this_opt] = parseFloat(arg);
      } else {
        opts[this_opt] = arg;
      }
      this_opt = null;
    } else {
      throw new Error(`Unexpected argument ${arg}`);
    }
  }
  i += 1;
}

console.log(opts)

await run(JSON.parse(argv[3]), opts);
