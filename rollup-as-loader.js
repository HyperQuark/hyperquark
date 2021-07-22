import asc from "assemblyscript/cli/asc";
import { readFileSync, writeFileSync } from "fs";
import { resolve as resolvePath } from "path";
import { parse as parseQueryString } from "query-string";

function resolveImport(id) {
  let code = readFileSync(id, { encoding: "utf-8" });
  code = code.replace(
    /(?:(?:import|export) +.+?from +(?:"|'))(.+?)(?:(?:"|');?$)/gm,
    (m, p) => {
      console.log(id, p, resolvePath(id, "." + p));
      return resolveImport(resolvePath(id, "." + p)) + "\n";
    }
  );
  return code;
}

async function load(id) {
  console.log("aaaaaa");
  if (!/\.(t|a)s(\?.*?)?$/.test(id)) return;
  let [fileId, query] = id.split("?");
  let compilerOptions = parseQueryString(query || "", {
    //  parseNumbers: true
  });
  for (let option in compilerOptions) {
    compilerOptions[option] ?? (compilerOptions[option] = true);
  }
  let z = await new Promise(async (resolve, reject) => {
    await asc.ready;
    // let code = readFileSync(fileId, { encoding: "utf-8" });
    let code = resolveImport(fileId);
    console.log(code);
    const { binary, text } = asc.compileString(code, compilerOptions);
    const moo =
      'import { instantiate as asInstantiate} from "@assemblyscript/loader";\
        export const instantiate = options => new Promise(async resolve => resolve(await asInstantiate(new Uint8Array([' +
      binary.toString() +
      "]), options)));\
          export const text = '" +
      text
        .replace(/\\/g, "\\\\")
        .replace(/'/g, "\\'")
        .replace(/\n/g, "\\n") +
      "';";
    console.log(moo);
    resolve({ code: moo });
  });
  return z;
}

export const assemblyScript = () => ({
  name: "assemblyscript",
  load
});
