<template>
  <span class="loading"><slot>Loading</slot></span>
</template>

<style scoped>
span.loading:after {
  content: "...";
  animation: loading 1s infinite;
}

@keyframes loading {
  0% {
    content: "";
  }
  25% {
    content: ".";
  }
  50% {
    content: "..";
  }
  75% {
    content: "...";
  }
}
</style>
