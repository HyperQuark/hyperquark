<template>
  <h1>{{ props.title || 'untitled' }}</h1>
  <span>by {{ props.author || 'unknown' }}</span>
  <input type="checkbox" name="turbo" :value="turbo"> <label for="turbo">turbo mode</label>
  <details v-if="error">
    <summary>An error occured whilst trying to load the project.</summary>
    {{ error }}
  </details>
  <template v-else>
    <br>
    <br>
    <button @click="greenFlag">green flag</button> <button>stop</button>
    <canvas width="480" height="360"></canvas>
    <div id="hq-output">Project output:<br></div>
  </template>
</template>

<script setup>
  import { sb3_to_wasm } from '@/../js/hyperquark.js';
  import { ref, nextTick } from 'vue';
  
  const props = defineProps(['json', 'title', 'author', 'assets']);
  let error = ref(null);
  let turbo = ref(false);
  let wasm;
  let start;
  try {
    wasm = sb3_to_wasm(JSON.stringify(props.json));
    start = eval(wasm);
    if (!typeof start === 'function') {
      throw start;
    }
  } catch (e) {
    error.value = e.toString();
    if (e.stack) {
      error.value += '\n' + e.stack;
    }
  }
  console.log(start);
  function greenFlag() {
    start({ framerate: turbo ? Infinity : 30 }).then(_=>alert('done')).catch(e => {
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