import { readFile, writeFile } from 'node:fs/promises';

let blocksRs = (await readFile('./src/ir/blocks.rs', 'utf8'));
const opcodes = [...new Set(blocksRs.match(/BlockOpcode::[a-z_]+? (?==>)/g).map(op => op.replace('BlockOpcode::', '').trim()))];
await writeFile('./js/opcodes.js', `export const opcodes = ${JSON.stringify(opcodes)};`);