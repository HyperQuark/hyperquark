<template>
  <Suspense>
    <ProjectIdPlayer :id="props.id"></ProjectIdPlayer>
    <template #fallback>
      <Loading></Loading>
    </template>
  </Suspense>
</template>

<script setup>
  import ProjectIdPlayer from '../components/ProjectIdPlayer.vue';
  import Loading from '../components/Loading.vue';
  
  const props = defineProps(['id']);
</script>