import asc from "assemblyscript/cli/asc";
import { readFileSync } from "fs";
import { parse as parseQueryString } from "query-string";

export function assemblyScript() {
  return {
    name: "assemblyscript",
    async load(id) {
      console.log("aaaaaa");
      if (!/\.(t|a)s(\?.*?)?$/.test(id)) return;
      let [fileId, query] = id.split("?");
      let compilerOptions = parseQueryString(query || "");
      for (let option in compilerOptions) {
        compilerOptions[option] ?? (compilerOptions[option] = true)
      }
      console.log(fileId, query, compilerOptions);
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
