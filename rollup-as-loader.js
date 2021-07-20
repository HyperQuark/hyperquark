const asc = require("assemblyscript/cli/asc");

export function assemblyScript () {
  return {
    name: "assemblyscript",
    transform (code, id) {
      if (!/\.as$/.test(id)) return;
      return new Promise(async (resolve, reject) => {
        await asc.ready;
        const { binary, text, stdout, stderr } = asc.compileString(code, {});
        resolve({
          code: `
          export const instantiate = options => new Promise(async resolve => resolve(await WebAssembly.instantiateStreaming(${binary}, options)));
          `
        });;
      });
    }
  }
}