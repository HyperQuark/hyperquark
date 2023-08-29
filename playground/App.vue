<script setup>
import { RouterLink, RouterView } from 'vue-router'
</script>

<template>
  <div class="wrapper">
    <nav>
      <RouterLink to="/"><img alt="HyperQuark logo" class="logo" src="/logo.png" />HyperQuark</RouterLink>
      <RouterLink to="/about">About</RouterLink>
      <a href="https://github.com/hyperquark/">Github</a>
    </nav>
  </div>
  <RouterView />
</template>

<style>
  header, main {
    text-align: center;
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
  font-size: 12px;
  text-align: left;
  margin-bottom: 1rem;
}

nav a.router-link-exact-active {
  color: var(--color-text);
}

nav a.router-link-exact-active:hover {
  background-color: transparent;
}

nav a {
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
</style>
