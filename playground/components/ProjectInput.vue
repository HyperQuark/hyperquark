<template>
  <details v-if="error">
    <summary>An error occured</summary>
    {{ error }}
  </details>
  <template v-else>
    <form class="inline-block" @submit.prevent="handleNumInput">
      <label for="project-id"
        >Enter a project ID:
        <span class="projinput inline-block"
          >https://scratch.mit.edu/projects/<input
            id="project-id"
            type="text"
            ref="numInput"
            v-model="projId"
            inputmode="numeric"
            required /></span
      ></label>
      <button type="submit" :disabled="goDisabled" :title="buttonTooltip">
        Go!
      </button>
    </form>
    <span class="inline-block"
      >or upload a project: <ProjectFileInput @error="err"></ProjectFileInput
    ></span>
  </template>
</template>

<script setup>
import { ref, watch } from "vue";
import { useRouter } from "vue-router";
import ProjectFileInput from "./ProjectFileInput.vue";

const router = useRouter();
const projId = ref("");
const numInput = ref(null);
const fileInput = ref(null);
const error = ref(null);
const goDisabled = ref(true);
const buttonTooltipText = "You need to enter a project ID";
const buttonTooltip = ref(buttonTooltipText);

watch(projId, (newVal) => {
  projId.value = newVal.toString().replaceAll(/[^\d]/g, "");
  goDisabled.value = projId.value === "";
  buttonTooltip.value = projId.value === "" ? buttonTooltipText : undefined;
});

function handleNumInput() {
  router.push({ name: "projectIdPlayer", params: { id: projId.value } });
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
