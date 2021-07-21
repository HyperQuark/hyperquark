import asc from "assemblyscript/cli/asc";
import { readFileSync } from "fs";

export function assemblyScript() {
  return {
    name: "assemblyscript",
    async load(id) {
      console.log("aaaaaa");
      if (!/\.(t|a)s$/.test(id)) return;
      let 
      let z = await new Promise(async (resolve, reject) => {
        await asc.ready;
        const { binary, text } = asc.compileString(
          readFileSync(fileId, { encoding: "utf-8" }),
          compilerOptions
        );
        const moo = `
          export const instantiate = options => new Promise(async resolve => resolve(await WebAssembly.instantiate(new Uint8Array([${binary.toString()}]), options)));
          `;
        console.log(moo);
        resolve({ code: moo });
      });
      return z;
    }
  };
}
