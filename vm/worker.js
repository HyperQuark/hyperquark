import { instantiate } from "./vm.ts";

self.addEventListener("message", ({ data }) => {
  self.postMessage(5)
});