import { readFile, writeFile } from "node:fs/promises";

let blocksRs = await readFile("./src/ir/blocks.rs", "utf8");
const opcodes = [
  ...new Set(
    blocksRs
      .match(/BlockOpcode::[a-zA-Z_0-9]+? (?==>)/g)
      .map((op) => op.replace("BlockOpcode::", "").trim()),
  ),
  "event_whenflagclicked",
  "event_whenbroadcastreceived",
  "event_whenthisspriteclicked",
  "event_whenstageclicked",
  "procedures_definition",
].sort();
await writeFile(
  "/tmp/hq-build/js/opcodes.js",
  `export const opcodes = ${JSON.stringify(opcodes)};`,
);
