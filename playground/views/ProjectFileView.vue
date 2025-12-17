<template>
  <details v-if="error">
    <summary>An error occured</summary>
    {{ error }}
  </details>
  <template v-else>
    <Suspense v-if="file.json !== null">
      <ProjectPlayer
        :json="file.json"
        :assets="file.assets"
        :title="file.title"
        :author="file.author"
        :zip="file.zip"
      ></ProjectPlayer>
      <template #fallback>
        <Loading>Loading...</Loading>
      </template>
    </Suspense>
    <div v-else>
      Upload a project: <ProjectFileInput @error="err"></ProjectFileInput>
    </div>
  </template>
</template>

<script setup>
import { ref } from "vue";
import ProjectFileInput from "../components/ProjectFileInput.vue";
import ProjectPlayer from "../components/ProjectPlayer.vue";
import { useProjectFileStore } from "../stores/projectfile.js";
import Loading from "../components/Loading.vue";

const file = useProjectFileStore();
const error = ref(null);

let err = (e) => (error.value = e);
</script>
