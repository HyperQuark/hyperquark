var CACHE_NAME = "hyperquark-cache-v1 ";
var urlsToCache = [];

self.addEventListener("install", function(event) {
  // Perform install steps
  event.waitUntil(
    caches.open(CACHE_NAME).then(function(cache) {
      // console.log("Opened cache");
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

self.addEventListener("fetch", async event => {
//  console.log(event.request);
  event.respondWith(customHeaderRequestFetch(event));
});
function customHeaderRequestFetch(event) {
  return new Promise((resolve, reject) => {
    fetch(event.request).then(response => {
    //  console.log(mapToObj(new Map(response.headers)));

      const newHeaders = new Headers(response.headers);
      newHeaders.append("Cross-Origin-Opener-Policy", "same-origin");
      newHeaders.append("Cross-Origin-Embedder-Policy", "require-corp");

      const anotherResponse = new Response(response.body, {
        status: response.status,
        statusText: response.statusText,
        headers: newHeaders
      });

    //  console.log(mapToObj(new Map(anotherResponse.headers)));
      resolve(anotherResponse);
    });
  });
}

function mapToObj(map) {
  const obj = {};
  for (let [k, v] of map) obj[k] = v;
  return JSON.stringify(obj);
}
