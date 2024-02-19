<template>
    <Suspense>
      <ProjectPlayer :json="json" title="HQ test project" author="HyperQuark"></ProjectPlayer>
      <template #fallback>
          <Loading></Loading>
      </template>
    </Suspense>
</template>

<script setup>
    import ProjectPlayer from '../components/ProjectPlayer.vue';
    import Loading from '../components/Loading.vue';
    import json from '@/../hq-test.project.json';
</script>