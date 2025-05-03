<template>
  <div v-show="loaded">
    <h1>{{ props.title || 'untitled' }}</h1>
    <span>by {{ props.author || 'unknown' }}</span>
    <input type="checkbox" id="turbo" :value="turbo"> <label for="turbo">turbo mode</label>
    <details v-if="error">
      <summary>{{ errorMode }} was emitted whilst {{ errorStage }} project.</summary>
      <span v-html="error"></span>
    </details>
    <!--<template v-else>-->
    <br>
    <br>
    <button @click="greenFlag">green flag</button> <button @click="stop">stop</button>
    <canvas width="480" height="360" ref="canvas"></canvas>
    <div id="hq-output">Project output:<br></div>
  </div>
  <Loading v-if='!loaded'>{{ loadingMsg }}</Loading>
</template>

<script setup>
import Loading from './Loading.vue';
import { sb3_to_wasm, FinishedWasm, WasmFlags } from '../../js/compiler/hyperquark.js';
import runProject from '../lib/project-runner.js';
import { ref, onMounted, nextTick } from 'vue';
import { getSettings } from '../lib/settings.js';
const Renderer = window.ScratchRender;
const props = defineProps(['json', 'title', 'author', 'assets', 'zip']);
let error = ref(null);
let errorStage = ref("loading");
let errorMode = ref("An error")
let turbo = ref(false);
let canvas = ref(null);
let loadingMsg = ref('loading assets');
let loaded = ref(false);
let assets = null;
let renderer;
let wasmProject;
let start;
const load_asset = async (md5ext) => {
  try {
    if (props.zip) {
      const file = props.zip.file(md5ext);
      const data = await file.async("text")//.then(console.log);
      //console.log(file, data);
      return data;
    }
    return await (await fetch(`https://assets.scratch.mit.edu/internalapi/asset/${md5ext}/get/`)).text();
  } catch (e) {
    error.value = `failed to load asset ${md5ext}\n${e.stack}`
  }
}
onMounted(() => {
  renderer = new Renderer(canvas.value);
});
//set_attr('load_asset', load_asset);
let wasmBytes;
let success;
try {
  // we need to convert settings to and from a JsValue because the WasmFlags exported from the
  // no-compiler version is not the same as that exported by the compiler... because reasons
  errorStage.value = "compiling";
  wasmProject = sb3_to_wasm(JSON.stringify(props.json), WasmFlags.from_js(getSettings().to_js()));
  console.log(wasmProject)
  if (!wasmProject instanceof FinishedWasm) {
    throw new Error("unknown error occurred when compiling project");
  }
  wasmBytes = wasmProject.wasm_bytes;
  if (getSettings().to_js().wasm_opt == 'On') {
    try {
      console.log('loading binaryen...');
      console.log(getSettings().to_js().scheduler)
      let binaryen = (await import('binaryen')).default; // only load binaryen if it's used.
      console.log('optimising using wasm-opt...')
      errorStage.value = "optimising";
      errorMode.value = "A warning";
      let binaryenModule = binaryen.readBinary(wasmBytes);
      console.log(wasmBytes.length);
      binaryenModule.setFeatures(binaryen.Features.All);
      binaryen.setOptimizeLevel(3);
      binaryen.setShrinkLevel(0);
      console.log(binaryenModule.emitBinary().length);
      binaryenModule.runPasses(["generate-global-effects"]);
      binaryenModule.optimize();
      console.log(binaryenModule.emitBinary().length);
      // binaryenModule.runPasses(["flatten", 'rereloop']);
      // console.log(binaryenModule.emitBinary().length);
      // binaryenModule.optimize();
      console.log(binaryenModule.emitBinary().length);
      binaryenModule.optimize();
      console.log(binaryenModule.emitBinary().length);
      wasmBytes = binaryenModule.emitBinary();
      console.log(wasmBytes.length)
    } catch (e) {
      console.error(e)
      error.value = e.message.toString();
      if (e.stack) {
        error.value += '<br>' + e.stack;
      }
      error.value += '<br>See browser console for more info.\
        <brThis is not an unrecoverable error; the project should play \
        as normal (possibly with worse-than-expected performance).';
      success = false;
    }
  } else {
    console.log('skipping wasm-opt due to user settings');
  }
  success = true;
} catch (e) {
  console.error(e);
  error.value = e.toString();
  if (e.stack) {
    error.value += '<br>' + e.stack;
  }
  success = false;
}
console.log(wasmProject)
Promise.all(
  props.json.targets.map(
    target => new Promise(
      r1 => Promise.all(
        target.costumes.map(
          ({ md5ext, dataFormat }) => new Promise(
            r2 => load_asset(md5ext).then(data => r2([md5ext, [dataFormat, data]]))
          )
        )
      ).then(r1)
    )
  )
).then(result => {
  assets = Object.fromEntries(result.flat());
  loaded.value = true;
});
function greenFlag() {
  if (!success) {
    return;
  }
  runProject({
    framerate: 30,
    renderer,
    turbo: turbo.value,
    wasm_bytes: wasmBytes,
    string_consts: wasmProject.strings,
    target_names: wasmProject.target_names,
    project_json: props.json,
    assets
  }).catch(e => {
    console.error(e);
    errorMode.value = "An error";
    errorStage.value = "running";
    error.value = e.toString();
    if (e.stack) {
      error.value += '\n' + e.stack;
    }
  });
}
function stop() {
  window.stop()
}
</script>

<style scoped>
canvas {
  border: 1px solid black;
  background: white;
  max-width: calc((100vw - 1rem) * 0.95);
}
</style>