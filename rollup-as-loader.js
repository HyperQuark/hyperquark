import { ready as ascReady, createMemoryStream, main as ascMain, options as ascOptions } from "assemblyscript/cli/asc";
import { readFileSync, writeFileSync } from "fs";
import { resolve as resolvePath } from "path";
import { parse as parseQueryString } from "query-string";

function resolveImport(id, cache) {
  let code = readFileSync(id, { encoding: "utf-8" });
//  code = code.replace(/\/\*.??\*\//gms, "");
//  code = code.replace(/\/\/.+$/gm, "");
  /*code = code.replace(
    /(?:(?:import|export) +.+?from +(?:"|'))(.+?)(?:(?:"|');?$)/gms,
    (m, p) => {
      if (/^\.\./.test(p)) p = "../" + p;
      else p = "." + p;
      if (!/\.$/.test(p)) p += ".ts";
      else p = p + "/index.ts";
    //  console.log(id, p, resolvePath(id, p));
      let absId = resolvePath(id, p);
      if (cache[absId]) return "";
      else {
        cache[absId] = true;
        return resolveImport(absId, cache) + "\n";
      }
    }
  );*/
  //code = code.replace(/\/\*.+?\*\//gms, "");
  return code.replace(/\s*?\n+\s*?/gm, "\n")
    // handle weird error edge cases
  //  .replace("export class ShadowRootInit", "class ShadowRootInit")
   // .replace("export declare function cloneNode", "declare function cloneNode")
 // .replace("ERROR('NodeList is not writable.')", "")
}

async function load(id) {
  let cache = {};
 // console.log("aaaaaa");
  if (!/\.(t|a)s(\?.*?)?$/.test(id)) return;
  let [fileId, query] = id.split("?");
  let compilerOptions = parseQueryString(query || "", {
    parseNumbers: true
  });
  console.log(compilerOptions);
  for (let option in compilerOptions) {
    compilerOptions[option] ?? (compilerOptions[option] = true);
  }
  console.log(compilerOptions);
  let z = await new Promise(async (resolve, reject) => {
    await ascReady;
    // let code = readFileSync(fileId, { encoding: "utf-8" });
    let code = resolveImport(fileId, cache);
   // writeFileSync("/app/built.ts", code, {encoding:"utf-8"});
   // console.log(code);
  //  console.log(/@ts-expect-error/.test(code));
      
      var { binary, text, stderr } = compileString(code, compilerOptions);
    
      if (stderr.length) console.error(stderr.toString());
    
    const moo =
      'import { instantiate as asInstantiate} from "@assemblyscript/loader";\
        export const instantiate = options => new Promise(async resolve => resolve(await asInstantiate(new Uint8Array([' +
      binary?.toString() +
      "]), options)));\
          export const text = '" +
      text
        ?.replace(/\\/g, "\\\\")
        ?.replace(/'/g, "\\'")
        ?.replace(/\n/g, "\\n") +
      "';";
  //  console.log(moo);
    resolve({ code: moo });
  });
  return z;
}

export const assemblyScript = () => ({
  name: "assemblyscript",
  load
});

const compileString = (sources, options) => {
  if (typeof sources === "string") sources = { "input.ts": sources };
  const output = Object.create({
    stdout: createMemoryStream(),
    stderr: createMemoryStream()
  });
  var argv = [
    "--binaryFile", "binary",
    "--textFile", "text",
  ];
  Object.keys(options || {}).forEach(key => {
    var val = options[key];
    var opt = ascOptions[key];
    if (opt && opt.type === "b") {
      if (val) argv.push("--" + key);
    } else {
      if (Array.isArray(val)) {
        val.forEach(val => { argv.push("--" + key, String(val)); });
      }
      else argv.push("--" + key, String(val));
    }
  });
  ascMain(argv.concat(Object.keys(sources)), {
    stdout: output.stdout,
    stderr: output.stderr,
    readFile: name => {
      if (name === "input.ts") return sources["input.ts"];
      try {
       return readFileSync("/app/" + name, { encoding: "utf-8" });
      } catch {
        return null;
      }
    },
    writeFile: (name, contents) => { output[name] = contents; },
    listFiles: () => []
  });
  return output;
};
