var CACHE_NAME = "hyperquark-cache-v1 ";
var urlsToCache = [];

self.addEventListener("install", function(event) {
  // Perform install steps
  event.waitUntil(
    caches.open(CACHE_NAME).then(function(cache) {
      console.log("Opened cache");
      return cache.addAll(urlsToCache);
    })
  );
});

self.addEventListener("activate", function(event) {
  var cacheAllowlist = [CACHE_NAME];

  event.waitUntil(
    caches.keys().then(function(cacheNames) {
      return Promise.all(
        cacheNames.map(function(cacheName) {
          if (cacheAllowlist.indexOf(cacheName) === -1) {
            return caches.delete(cacheName);
          }
        })
      );
    })
  );
});

self.addEventListener("fetch", event => {
  event.respondWith(customHeaderRequestFetch(event));
});
function customHeaderRequestFetch(event) {
  // decide for yourself which values you provide to mode and credentials
  const newRequest = new Request(event.request, {
    mode: "cors",
    credentials: "omit",
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp"
    }
  });
  return fetch(newRequest);
}
