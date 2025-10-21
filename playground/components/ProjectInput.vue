<template>
   <details v-if="error">
    <summary>An error occured</summary>
    {{error}}
  </details>
  <template v-else>
    <span class=".inline-block">Enter a project id: <span class="projinput inline-block" tabindex=0 @focus="()=>numInput.focus()">https://scratch.mit.edu/projects/<input type="text" ref="numInput" v-model="projId"></span><button @click="handleNumInput" :disabled="goDisabled">Go!</button></span> <span class="inline-block">or upload a project: <ProjectFileInput @error="err"></ProjectFileInput></span>
  </template>
</template>

<script setup>
  import { ref, watch } from "vue";
  import { useRouter } from 'vue-router';
  import ProjectFileInput from './ProjectFileInput.vue'
  
  const router = useRouter();
  const projId = ref("");
  const numInput = ref(null);
  const fileInput = ref(null);
  const error = ref(null);
  const goDisabled = ref(true);
  watch(projId, (newVal) => {
    projId.value = newVal.toString().replaceAll(/[^\d]/g, '');
    goDisabled.value = projId.value === "";
  });
  
  function handleNumInput() {
    router.push({ name: 'projectIdPlayer', params: { id: projId.value }})
  }
  
  function err(e) {
    error.value = e;
  }
</script>

<style scoped>
  .inline-block {
    display: inline-block;
  }
  
  input[type="text"] {
    width: 12ch;
    color: var(--color-text);
    background: var(--color-background);
    border: 0;
    :focus {
      outline: none;
    }
  }
  
  .projinput {
    font-family: monospace;
    font-size: 12px;
    padding: 0;
    border: var(--color-border) solid 1px;
  }
</style>
