<script setup>
import { RouterLink, RouterView, useRoute } from 'vue-router';
import { useDebugModeStore } from './stores/debug';
const debugModeStore = useDebugModeStore();
const route = useRoute();

const is_prod = window.location.host === "hyperquark.edgecompute.app";
const hq_version = import.meta.env.VITE_HQ_VERSION_NAME;
</script>

<template>
  <div v-if="!is_prod" id="dev-banner">This is a development preview of HyperQuark. Please find the most recent stable version at <a href="https://hyperquark.edgecompute.app">https://hyperquark.edgecompute.app</a></div>.
  <div class="wrapper">
    <nav>
      <RouterLink to="/"><img alt="HyperQuark logo" class="logo" src="/logo.png" />HyperQuark <span v-if="!!hq_version" id="hq-version">{{ hq_version }}</span></RouterLink>
      <RouterLink to="/about">About</RouterLink>
      <a href="https://github.com/hyperquark/">Github</a>
      <RouterLink to="/settings">Settings</RouterLink>
      <a class="fake-link" @click="debugModeStore.toggleDebug">{{ debugModeStore.debug ? 'disable' : 'enable' }} debug mode</a>
    </nav>
  </div>
  <RouterView />
</template>

<style>
  header, main {
    text-align: center;
  }

  #dev-banner {
    text-align: center;
    border: 3px dashed #0d79f5;
    padding: 0.5em;
    margin: 0;
    margin-bottom: 0.5em;
    position: sticky;
    top: 5px;
    background-color: var(--color-background);
  }
</style>

<style scoped>
.logo {
  vertical-align: middle;
  display: inline-block;
}

nav {
  .logo {
    width: 30px;
    height: 30px;
  }
  width: 100%;
  font-size: 0.9em;
  text-align: center;
  margin-bottom: 1rem;
}

span#hq-version {
  font-size: 0.8em;
}

nav a.router-link-exact-active {
  color: var(--color-text);
}

nav a.router-link-exact-active:hover, nav .fake-link:hover {
  background-color: transparent;
}

nav a, nav .fake-link {
  display: inline-block;
  padding: 0 1rem;
  border-left: 1px solid var(--color-border);
}

nav a:first-of-type {
  border: 0;
}

div.wrapper {
  margin: 0 auto;
}

.fake-link {
  cursor: pointer;
}
</style>
