<template>
  <ProjectPlayer v-if="success === true" :json="json"></ProjectPlayer>
  <template v-else>
    <h1>Project not found</h1>
    This could be because it doesn't exist, or it may be private.
    <br>
    <details>
      <summary>Error message (probably unhelpful):</summary>
      {{ success }}
    </details>    
  </template>
</template>

<script setup>
  import ProjectPlayer from './ProjectPlayer.vue';
  import { ref } from 'vue';
  
  const props = defineProps(['id'])
  const success = ref(null);
  const json = ref("");
  try {
    const apiRes = await (await fetch(`https://trampoline.turbowarp.org/api/projects/${props.id}/`)).json()//.project_token;
    json.value = await fetch(`https://projects.scratch.mit.edu/${props.id}/?token=${apiRes.project_token}`).then(res => {
      if (!res.ok) {
        throw new Error("response was not OK");
      }
      return res.json()
    });
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