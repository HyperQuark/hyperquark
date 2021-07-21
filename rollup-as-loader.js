const asc = require("assemblyscript/cli/asc");

export function assemblyScript () {
  return {
    name: "assemblyscript",
    transform (code, id) {
      console.log("aaaaaa");
      if (!/\.ts$/.test(id)) return;
      return new Promise(async (resolve, reject) => {
        await asc.ready;
        const { binary, text, stdout, stderr } = asc.compileString(code, asc.options);
        const moo = `
          export const instantiate = options => new Promise(async resolve => resolve(await WebAssembly.instantiate(new Uint8Array([${binary.toString()}]), options)));
          `;
        console.log(moo);
        resolve({code: moo});
      });
    }
  }
}