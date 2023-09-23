<template>
  <h1>{{ props.title || 'untitled' }}</h1>
  <span>by {{ props.author || 'unknown' }}</span>
  <input type="checkbox" id="turbo" :value="turbo"> <label for="turbo">turbo mode</label>
  <details v-if="error">
    <summary>An error occured whilst trying to load the project.</summary>
    {{ error }}
  </details>
  <template v-else>
    <br>
    <br>
    <button @click="greenFlag">green flag</button> <button>stop</button>
    <canvas width="480" height="360" ref="canvas"></canvas>
    <div id="hq-output">Project output:<br></div>
  </template>
</template>

<script setup>
  import { sb3_to_wasm, WasmProject } from '@/../js/hyperquark.js';
  import runProject from '@/lib/project-runner.js';
  import { ref, onMounted } from 'vue';
  const Renderer = window.ScratchRender;
  const props = defineProps(['json', 'title', 'author', 'assets']);
  let error = ref(null);
  let turbo = ref(false);
  let canvas = ref(null);
  let renderer;
  let wasmProject;
  let start;
  onMounted(() => {
    renderer = new Renderer(canvas.value);
  });
  try {
    wasmProject = sb3_to_wasm(JSON.stringify(props.json));
    console.log(wasmProject)
    if (!wasmProject instanceof WasmProject) {
      throw new Error("unknown error occurred when compiling project");
    }
  } catch (e) {
    error.value = e.toString();
    if (e.stack) {
      error.value += '\n' + e.stack;
    }
  }
  function greenFlag() {
    runProject({ framerate: turbo ? Infinity : 30, renderer, ...wasmProject }).then(_=>alert('done')).catch(e => {
      error.value = e.toString();
      if (e.stack) {
        error.value += '\n' + e.stack;
      }
    });
  }
</script>

<style scoped>
  canvas {
    border: 1px solid black;
    background: white;
    max-width: calc((100vw - 1rem) * 0.95);
  }
</style>