<template>
  todo
  <br>
  <!--<div>
    {{wasm}}
  </div>-->
  <div id="hq-output"></div>
</template>

<script setup>
  import { sb3_to_wasm } from '@/../js/hyperquark.js';
  import { ref, nextTick } from 'vue';
  
  const props = defineProps(['json', 'assets']);
  let wasm = ref("");
  try {
    wasm.value = sb3_to_wasm(JSON.stringify(props.json));
  } catch (e) {
    wasm.value = e;
  }
  nextTick(_ => eval(wasm.value));
</script>