<template>
  <div class="about">
    <header>
      <h1>This is an about page.</h1>
    </header>
    <main>
      <p>
        HyperQuark compiles Scratch projects to <a href="https://webassembly.org">WebAssembly</a> in an attempt to make
        them faster.
        It was created by <a href="https://github.com/pufferfish101007">pufferfish101007</a> and was written in <a
          href="https://rust-lang.org">Rust</a> mainly, with the exception of this website which uses <a
          href="https://vuejs.org">Vue</a>.
        The source code is available <a href="https://github.com/hyperquark">on Github</a>.
      </p>
    </main>
  </div>
</template>

<style>
@media (min-width: 1024px) {
  .about {
    min-height: 100vh;
    display: flex;
    align-items: center;
    flex-direction: column;
    text-align: center;
  }
}
</style>
