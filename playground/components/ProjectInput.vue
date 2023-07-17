<template>
  <span class=".inline-block">Enter a project id: <span class="projinput inline-block" tabindex=0 @focus="()=>numInput.focus()">https://scratch.mit.edu/projects/<input type="number" min="1" placeholder="771449498" ref="numInput" v-model="projId"></span><button @click="handleNumInput">Go!</button></span> <span class="inline-block">or upload a project: <input type="file" ref="fileInput" @input="handleFileInput"></span>
</template>

<script setup>
  import { ref } from "vue";
  import { useRouter } from 'vue-router';
  
  const router = useRouter();
  const projId = ref("");
  const numInput = ref(null);
  const fileInput = ref(null);
  
  function handleNumInput() {
    if (!/^\d+$/.test(projId.value)) {
      return alert("invalid project id");
    }
    router.push({ name: 'projectPlayerId', params: { id: projId.value }})
  }
  
  function handleFileInput() {
    alert("not implemented yet");
  }
</script>

<style scoped>
  .inline-block {
    display: inline-block;
  }
  
  input[type="number"] {
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