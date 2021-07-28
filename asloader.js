import {
  ready as ascReady,
  createMemoryStream,
  main as ascMain,
  options as ascOptions
} from "assemblyscript/cli/asc";
import { readFileSync, writeFileSync } from "fs";
import { resolve as resolvePath } from "path";
import { parse as parseQueryString } from "query-string";
import { cwd } from "process";

async function load(id) {
  let cache = {};
  // console.log("aaaaaa");
  if (!/\.(t|a)s(\?.*?)?$/.test(id)) return;
  let [fileId, query] = id.split("?");
  let compilerOptions = parseQueryString(query || "", {
    parseNumbers: true
  });
  //  console.log(compilerOptions);
  for (let option in compilerOptions) {
    compilerOptions[option] ?? (compilerOptions[option] = true);
  }
  //  console.log(compilerOptions);
  let z = await new Promise(async (resolve, reject) => {
    await ascReady;
    let code = readFileSync(fileId, { encoding: "utf-8" });
    var { binary, text, stderr } = compileString(code, compilerOptions);

    if (stderr.length) console.error(stderr.toString());
    
    const moo =
      'import { instantiate as asInstantiate} from "@assemblyscript/loader";\n' +
        'export const binary = new Uint8Array([' +
      binary?.toString() +
      "]);\n" +
        "export const instantiate = options => new Promise(async resolve => resolve(await asInstantiate(binary, options)));\
          export const text = '" +
      text
        ?.replace(/\\/g, "\\\\")
        ?.replace(/'/g, "\\'")
        ?.replace(/\n/g, "\\n") +
      "';";
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
  var argv = ["--binaryFile", "binary", "--textFile", "text"];
  Object.keys(options || {}).forEach(key => {
    var val = options[key];
    var opt = ascOptions[key];
    if (opt && opt.type === "b") {
      if (val) argv.push("--" + key);
    } else {
      if (Array.isArray(val)) {
        val.forEach(val => {
          argv.push("--" + key, String(val));
        });
      } else argv.push("--" + key, String(val));
    }
  });
  
  ascMain(argv.concat(Object.keys(sources)), {
    stdout: output.stdout,
    stderr: output.stderr,
    readFile: name => {
      if (name === "input.ts") return sources["input.ts"];
      try {
        return readFileSync(cwd() + "/" + name, { encoding: "utf-8" });
      } catch {
        return null;
      }
    },
    writeFile: (name, contents) => {
      output[name] = contents;
    },
    listFiles: () => []
  });
  return output;
};
