<template>
  <h1>{{ props.title || 'untitled' }}</h1>
  <span>by {{ props.author || 'unknown' }}</span>
  <br>
  <br>
  <button @click="start({ framerate: 30 })">green flag</button> <button>stop</button>
  <canvas width="480" height="360"></canvas>
  <div id="hq-output">Project output:<br></div>
</template>

<script setup>
  import { sb3_to_wasm } from '@/../js/hyperquark.js';
  import { ref, nextTick } from 'vue';
  
  const props = defineProps(['json', 'title', 'author', 'assets']);
  let wasm;
  let start;
  try {
    wasm = sb3_to_wasm(JSON.stringify(props.json));
  } catch (e) {
    wasm = e;
  }
  start = eval(wasm);
  console.log(start);
</script>

<style scoped>
  canvas {
    border: 1px solid black;
    background: white;
    width: calc((100vw - 1rem) * 0.95);
  }
</style>