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
    <br>
    <br>
    <br>
    <h2>HyperQuark featured projects</h2>
    <br>
    <ul id="featured-projects-container">
      <li v-for="project in featuredProjects">
        <a :href="'/hyperquark/projects/' + project.id">{{ project.name }}</a> by {{ project.author }}
      </li>
    </ul>
  </main>
</template>

<script>
  // todo: just use ids, dynamically fetch/cache title, author thumbnail etc
  // or use a studio like turbowarp
  // but at the moment this is kind of just a quick-access bar for frequently tested projects
  const featuredProjects = [
    {
      id: 951679565,
      name: 'Colourful Smiley Face Simulator',
      author: 'pufferfish_test',
    },
    {
      id: 886414106,
      name: 'benchmark',
      author: 'pufferfish_test',
    }
  ];
</script>

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
  ul#featured-projects-container {
    list-style: none;
    padding: inherit;
  }
</style>