import { Buffer } from "node:buffer";
import validate from "scratch-parser";
import { SB1File, ValidationError } from "scratch-sb1-converter";

// adapted from https://github.com/scratchfoundation/scratch-vm/blob/6cfea59e7aaff880a1c4e709b2adc14d0113ecab/src/virtual-machine.js#L320
/**
 * @param {ArrayBuffer|String} input
 */
export const unpackProject = (input) => {
  if (typeof input !== "string") {
    input = Buffer.from(input);
  }
  return new Promise((resolve, reject) => {
    // The second argument of false below indicates to the validator that the
    // input should be parsed/validated as an entire project (and not a single sprite)
    validate(input, false, (error, res) => {
      if (error) return reject(error);
      resolve(res);
    });
  })
    .catch((error) => {
      try {
        const sb1 = new SB1File(input);
        const json = sb1.json;
        json.projectVersion = 2;
        return Promise.resolve([json, sb1.zip]);
      } catch (sb1Error) {
        if (sb1Error instanceof ValidationError) {
          // The input does not validate as a Scratch 1 file.
        } else {
          // The project appears to be a Scratch 1 file but it
          // could not be successfully translated into a Scratch 2
          // project.
          return Promise.reject(sb1Error);
        }
      }
      // Throw original error since the input does not appear to be
      // an SB1File.
      return Promise.reject(error);
    })
    .then(async ([json, zip]) => {
      if (json.projectVersion === 3) {
        return [json, zip];
      }
      if (json.projectVersion === 2) {
        const { default: ScratchVM } = await import("@scratch/scratch-vm");
        // const scratchVm =
        //   typeof window === "object"
        //     ? await import("@scratch/scratch-vm/dist/web/scratch-vm.js")
        //     : await import("@scratch/scratch-vm/dist/node/scratch-vm.js");
        const VM = new ScratchVM();
        await VM.deserializeProject(json, zip);
        VM.runtime.handleProjectLoaded();
        const sb3Json = JSON.parse(VM.toJSON());
        return [sb3Json, zip];
      }
      throw "Unable to verify Scratch Project version";
    });
};
