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
        <RouterLink :to="'projects/' + project.id">{{ project.name }}</RouterLink> by {{ project.author }}
      </li>
    </ul>
    <br>
    <h2>Currently implemented blocks:</h2>
    <ul id="implemented-opcodes-container">
      <li v-for="opcode in implementedOpcodes" :class="'block-category-' + opcode.match(/^[a-z]+(?=_)/)">
        {{ opcode }}
      </li>
    </ul>
  </main>
</template>

<script>
  import { opcodes as implementedOpcodes } from '@/../js/opcodes.js';
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
    },
    {
      id: 1050568991,
      name: 'fibonacci benchmark',
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
  ul {
    list-style: none;
    padding: inherit;
  }
  li.block-category-looks {
    color:#9966FF;
  }
  li.block-category-operator {
    color: #59C059;
  }
  li.block-category-motion {
    color: #4C97FF;
  }
  li.block-category-control {
    color: #FFAB19;
  }
  li.block-category-pen {
    color: #0fBD8C;
  }
  li.block-category-data {
    color: #FF8C1A;
  }
</style>