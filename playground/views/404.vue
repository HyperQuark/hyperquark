<template>
  <header><h1>404 Not Found</h1></header>
  <main>
    <p>This page doesn't exist.</p>
    <p><RouterLink to="/">Go home</RouterLink></p>
  </main>
</template>

<script setup>
import { RouterLink } from "vue-router";
</script>
