<template>
  <ProjectPlayer
    v-if="success === true"
    :json="json"
    :id="props.id"
    :author="author"
    :title="title"
    :instructions="instructions"
    :description="description"
    :zip="zip"
  ></ProjectPlayer>
  <template v-else>
    <h1>Project not found</h1>
    This could be because it doesn't exist, or it may be private.
    <br />
    <details>
      <summary>Error message (probably unhelpful):</summary>
      {{ success }}
    </details>
  </template>
</template>

<script setup>
import { unpackProject } from "../lib/project-loader";
import ProjectPlayer from "./ProjectPlayer.vue";
import { ref, nextTick } from "vue";

const props = defineProps(["id"]);
const success = ref(null);
const json = ref("");
const zip = ref("");
const title = ref("");
const author = ref("");
const instructions = ref("");
const description = ref("");
try {
  const apiRes = await (
    await fetch(`https://trampoline.turbowarp.org/api/projects/${props.id}/`)
  ).json();
  title.value = apiRes.title;
  author.value = apiRes.author.username;
  instructions.value = apiRes.instructions;
  description.value = apiRes.description;
  const res = await fetch(
    `https://projects.scratch.mit.edu/${props.id}?token=${apiRes.project_token}`,
  );
  if (!res.ok) {
    throw new Error("response was not OK");
  }
  const [_json, _zip] = await unpackProject(await res.arrayBuffer());
  zip.value = _zip;
  json.value = _json;
  success.value = true;
} catch (e) {
  success.value = e;
}
</script>

<style scoped>
details {
  margin-top: 1rem;
  font-size: 12px;
}
</style>
