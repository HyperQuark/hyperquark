const asc = require("assemblyscript/cli/asc");

export function assemblyScript () {
  return {
    name: "assemblyscript",
    async transform (code, id) {
      console.log("aaaaaa");
      if (!/\.(t|a)s$/.test(id)) return;
      let z = await new Promise(async (resolve, reject) => {
        await asc.ready;
        const { binary, text } = asc.compileString(code);
        const moo = `
          export const instantiate = options => new Promise(async resolve => resolve(await WebAssembly.instantiate(new Uint8Array([${binary.toString()}]), options)));
          `;
        console.log(moo);
        resolve({code: moo});
      });
      return z;
    }
  }
}