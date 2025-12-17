<template>
  <Loading v-if="loading"></Loading
  ><input v-else type="file" ref="fileInput" @input="handleFileInput" />
</template>

<script setup>
import { ref } from "vue";
import { useRouter } from "vue-router";
import { unpackProject } from "../lib/project-loader.js";
import { useProjectFileStore } from "../stores/projectfile.js";
import Loading from "../components/Loading.vue";

const emit = defineEmits(["error"]);

const fileInput = ref(null);
const loading = ref(false);
const router = useRouter();
const fileStore = useProjectFileStore();

async function handleFileInput() {
  loading.value = true;
  console.log(fileInput);
  let file = fileInput.value.files[0];
  console.log(file);
  //console.log(Buffer.from(file.arrayBuffer()));
  //console.log(file.arrayBuffer());
  if (!file) {
    emit("error", "file doesn't exist");
    loading.value = false;
    return;
  }
  let json, zip, res;
  try {
    //console.log(Buffer.from(file.arrayBuffer()));
    res = await unpackProject(await file.arrayBuffer());
    [json, zip] = res;
  } catch (e) {
    console.log("error", e);
    emit(
      "error",
      e.hasOwnProperty("validationError")
        ? JSON.stringify(e)
        : e.toString() + (e.stack ? "\n" + e.stack : ""),
    );
    loading.value = false;
    return;
  }
  console.log(res, json, zip);
  fileStore.json = json;
  fileStore.title = file.name.replace(/\..+?$/, "");
  fileStore.zip = zip;
  router.push({ name: "projectFilePlayer" });
}
</script>
