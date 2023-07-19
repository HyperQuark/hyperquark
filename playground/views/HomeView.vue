<script setup>
import ProjectInput from '../components/ProjectInput.vue'
</script>

<template>
  <header>
    <h1>HyperQuark</h1>
  </header>
  <main>
    <div>HyperQuark compiles Scratch projects to WebAssembly in order to run them <span id="blazing">⚡ blazingly fast ⚡</span> really quickly.</div>
    <div><RouterLink to="/about">Learn more</RouterLink></div>
    <ProjectInput/>
  </main>
</template>

<style scoped>
  h1 {
    font-size: 2rem;
    font-weight: 500;
    margin-bottom: 1rem;
  }
  span#blazing {
    font-style: italic;
    text-decoration: line-through;
  }
</style>