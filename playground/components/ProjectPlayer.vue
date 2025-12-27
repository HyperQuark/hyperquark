<template>
  <div v-show="loaded">
    <h1>{{ props.title || "untitled" }}</h1>
    <span>by {{ props.author || "unknown" }}</span>
    <div v-if="!!id">
      <a :href="`https://scratch.mit.edu/projects/${props.id}/`"
        >View on Scratch</a
      >
      | <a :href="`https://turbowarp.org/${props.id}/`">View on TurboWarp</a>
    </div>
    <details v-if="error">
      <summary>
        {{ errorMode }} was emitted whilst {{ errorStage }} project.
      </summary>
      <span v-html="error"></span>
    </details>
    <div>
      <button @click="greenFlag">green flag</button>
      <button @click="stop">stop</button>
      <input type="checkbox" id="turbo" :value="turbo" />
      <label for="turbo">turbo mode</label>
    </div>
    <canvas width="480" height="360" ref="canvas"></canvas>
    <div class="instructions">
      <div v-if="props.instructions">
        <h2>Instructions</h2>
        {{ props.instructions }}
      </div>
      <div v-if="props.description">
        <h2>Notes and credits</h2>
        {{ props.description }}
      </div>
    </div>
  </div>
  <Loading v-if="!loaded">{{ loadingMsg }}</Loading>
</template>

<script setup>
import Loading from "./Loading.vue";
import {
  sb3_to_wasm,
  FinishedWasm,
  WasmFlags,
} from "../../js/compiler/hyperquark.js";
import { instantiateProject } from "../lib/project-runner.js";
import { ref, onMounted, registerRuntimeCompiler } from "vue";
import { getSettings } from "../lib/settings.js";
import { useDebugModeStore } from "../stores/debug.js";

const debugModeStore = useDebugModeStore();

const props = defineProps([
  "json",
  "title",
  "author",
  "assets",
  "zip",
  "instructions",
  "description",
  "id",
]);

let error = ref(null);
let errorStage = ref("loading");
let errorMode = ref("An error");
let turbo = ref(false);
let canvas = ref(null);
let loadingMsg = ref("compiling project");
let loaded = ref(false);

let greenFlag = () => null;
let stop = () => null;
let success = true;

const declareError = (e, terminate, mode, stage, extra) => {
  console.error(e);
  errorMode.value = mode;
  errorStage.value = stage;
  error.value = e.toString();
  if (e.stack) {
    error.value += "<br>" + e.stack;
  }
  if (extra) {
    error.value += extra;
  }
  if (terminate) {
    success = false;
    loaded.value = true;
  }
};

onMounted(async () => {
  const load_asset = async (md5ext) => {
    try {
      if (props.zip) {
        console.log(props.zip);
        const file = props.zip.file(md5ext) ?? props.zip.files[md5ext];
        const data = await file.async("text"); //.then(console.log);
        //console.log(file, data);
        return data;
      }
      return await (
        await fetch(
          `https://assets.scratch.mit.edu/internalapi/asset/${md5ext}/get/`,
        )
      ).text();
    } catch (e) {
      error.value = `failed to load asset ${md5ext}\n${e.stack}`;
    }
  };

  let wasmBytes;
  let assets = null;
  let wasmProject;

  try {
    // we need to convert settings to and from a JsValue because the WasmFlags exported from the
    // no-compiler version is not the same as that exported by the compiler... because reasons
    wasmProject = sb3_to_wasm(
      JSON.stringify(props.json),
      WasmFlags.from_js(getSettings().to_js()),
    );

    if ((!wasmProject) instanceof FinishedWasm) {
      throw new Error("unknown error occurred when compiling project");
    }

    wasmBytes = wasmProject.wasm_bytes;
  } catch (e) {
    declareError(e, true, "An error", "compiling");
  }

  if (!success) return;

  if (getSettings().to_js().wasm_opt == "On") {
    try {
      loadingMsg.value = "optimising project";
      console.log(getSettings().to_js().scheduler);
      const binaryen = (await import("binaryen")).default; // only load binaryen if it's used.
      const binaryenModule = binaryen.readBinary(wasmBytes);
      binaryenModule.setFeatures(binaryen.Features.All);
      binaryen.setOptimizeLevel(3);
      binaryen.setShrinkLevel(0);
      binaryenModule.runPasses(["generate-global-effects"]);
      binaryenModule.optimize();
      binaryenModule.optimize();
      wasmBytes = binaryenModule.emitBinary();
    } catch (e) {
      declareError(
        e,
        false,
        "A warning",
        "optimising",
        "<br>See browser console for more info.\
        <brThis might not be an unrecoverable error; the project may play \
        as normal (possibly with worse-than-expected performance).",
      );
    }
  } else {
    console.log("skipping wasm-opt due to user settings");
  }

  try {
    loadingMsg.value = "loading assets";
    const assetsResult = await Promise.all(
      props.json.targets.map(
        (target) =>
          new Promise((r1) =>
            Promise.all(
              target.costumes.map(
                ({ md5ext, dataFormat }) =>
                  new Promise((r2) =>
                    load_asset(md5ext).then((data) =>
                      r2([md5ext, { dataFormat, data }]),
                    ),
                  ),
              ),
            ).then(r1),
          ),
      ),
    );
    assets = Object.fromEntries(assetsResult.flat());
  } catch (e) {
    declareError(e, false, "A warning", "loading assets for");
  }

  try {
    loadingMsg.value = "instantiating project";
    const runner = await instantiateProject({
      framerate: 30,
      turbo: turbo.value,
      wasm_bytes: wasmBytes,
      strings: wasmProject.strings,
      target_names: wasmProject.target_names,
      project_json: props.json,
      assets,
      makeRenderer: async () => {
        const scratch_render =
          await import("scratch-render/dist/web/scratch-render.js");
        const RenderWebGL = scratch_render.default;
        return new RenderWebGL(canvas.value);
      },
      isDebug: () => debugModeStore.debug,
    });

    loaded.value = true;

    greenFlag = runner.greenFlag;
    stop = runner.stop;
  } catch (e) {
    declareError(e, true, "An error", "instantiating");
  }
});
</script>

<style scoped>
canvas {
  border: 1px solid black;
  background: white;
  max-width: calc((100vw - 1rem) * 0.95);
  float: left;
  margin-right: 1em;
  margin-bottom: 1.5em;
}

div.instructions {
  border-radius: 1em;
  border: 2px solid var(--color-border);
  padding: 1em;
  margin: 1em;
  width: fit-content;
  max-width: calc((100vw - 1rem) * 0.95);
  float: none;
  overflow: auto;
  min-width: 0;

  & > h2 {
    font-weight: bold;
  }

  white-space: pre-wrap;
}
</style>
