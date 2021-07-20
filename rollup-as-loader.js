const asc = require("assemblyscript/cli/asc");

export default function () {
  return {
    name: "assemblyscript",
    transform (code, id) {
      if (!/\.as$/.test(id)) return;
      return new Promise(async (resolve, reject) {
        await asc.ready();
        const { binary, text, stdout, stderr } = asc.compileString(code, {});
      });
    }
  }
}