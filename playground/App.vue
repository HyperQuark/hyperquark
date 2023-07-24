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

@media (min-width: 1024px) {
  header {
    display: flex;
    place-items: center;
    padding-right: calc(var(--section-gap) / 2);
  }

  .logo {
    /*margin: 0 2rem 0 0;*/
  }

  nav .wrapper {
    display: flex;
    place-items: flex-start;
    flex-wrap: wrap;
  }

  nav {
    text-align: left;
    margin-left: -1rem;
    font-size: 1rem;

    padding: 1rem 0;
    margin-top: 1rem;
  }
}
</style>
