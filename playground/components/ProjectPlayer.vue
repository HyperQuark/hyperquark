<template>
 <div v-show="loaded">
    <h1>{{ props.title || 'untitled' }}</h1>
    <span>by {{ props.author || 'unknown' }}</span>
    <input type="checkbox" id="turbo" :value="turbo"> <label for="turbo">turbo mode</label>
    <details v-if="error">
      <summary>An error occured whilst trying to load the project.</summary>
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
  import { sb3_to_wasm, /*set_attr*/ } from '@/../js/hyperquark.js';
  import runProject from '@/lib/project-runner.js';
  import { ref, onMounted, nextTick } from 'vue';
  const Renderer = window.ScratchRender;
  const props = defineProps(['json', 'title', 'author', 'assets', 'zip']);
  let error = ref(null);
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
  try {
    wasmProject = sb3_to_wasm(JSON.stringify(props.json));
    if (!wasmProject instanceof Uint8Array) {
      throw new Error("unknown error occurred when compiling project");
    }
    let assert = (bool) => {
        if (!bool) {
          throw new AssertionError("Assertion failed");
        }
      };
    assert(WebAssembly.validate(wasmProject));
    WebAssembly.instantiate(wasmProject, {
      looks: {
        say_float: console.log
      }
    }).then(inst => window.wasm = inst);
  } catch (e) {
    error.value = e.toString();
    if (e.stack) {
      error.value += '\n' + e.stack;
    }
  }
  loaded.value = true;
  /*Promise.all(
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
  });*/
  function greenFlag() {
    runProject({
      framerate: turbo ? Infinity : 30,
      renderer,
      wasm_bytes: wasmProject.wasm_bytes,
      string_consts: wasmProject.string_consts,
      target_names: wasmProject.target_names,
      project_json: props.json,
      assets
    }).catch(e => {
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