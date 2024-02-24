<template>
  <details v-if="error">
    <summary>An error occured</summary>
    {{error}}
  </details>
  <template v-else>
    <ProjectPlayer v-if="file.json !== null" :json="file.json" :assets="file.assets" :title="file.title" :author="file.author" :zip="file.zip"></ProjectPlayer>
    <div v-else>Upload a project: <ProjectFileInput @error="err"></ProjectFileInput></div>
  </template>
</template>

<script setup>
  import { ref } from 'vue';
  import ProjectFileInput from '../components/ProjectFileInput.vue';
  import ProjectPlayer from '../components/ProjectPlayer.vue';
  import { useProjectFileStore } from '../stores/projectfile.js';
  
  const file = useProjectFileStore();
  const error = ref(null);
  
  let err = e => error.value = e;
</script>