import asc from "assemblyscript/cli/asc";
import { readFileSync, writeFileSync } from "fs";
import { parse as parseQueryString } from "query-string";

export function assemblyScript() {
  return {
    name: "assemblyscript",
    async load(id) {
      console.log("aaaaaa");
      if (!/\.(t|a)s(\?.*?)?$/.test(id)) return;
      let [fileId, query] = id.split("?");
      let compilerOptions = parseQueryString(query || "", {
      //  parseNumbers: true
      });
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
        writeFileSync("/app/aa.md", text, { encoding: "utf-8" });
        const moo = "import { instantiate as asInstantiate} from \"@assemblyscript/loader\";\
        export const instantiate = options => new Promise(async resolve => resolve(await asInstantiate(new Uint8Array([" + binary.toString() + "]), options)));\
          export const text = '" + text.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\n/g, '\\n') + "';";
        console.log(moo);
        resolve({ code: moo });
      });
      return z;
    }
  };
}
