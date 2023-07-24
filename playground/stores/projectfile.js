import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useProjectFileStore = defineStore('projectFile', () => {
  const json = ref(null);
  const assets = ref([]);
  const title = ref('untitled');
  const author = ref('unknown');

  return { json, assets, title, author };
})
