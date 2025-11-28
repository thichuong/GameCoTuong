const CACHE_NAME = 'xiangqi-pwa-v1';
const ASSETS = [
    '/',
    '/index.html',
    '/manifest.json',
    '/icon.png',
    // Trunk will inject the WASM and JS filenames, but since they change hash, 
    // we might need a more robust strategy or just cache everything.
    // For a simple PWA, caching 'fetch' requests is often enough.
];

self.addEventListener('install', (e) => {
    e.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            return cache.addAll(ASSETS).catch(err => console.error(err));
        })
    );
});

self.addEventListener('fetch', (e) => {
    e.respondWith(
        caches.match(e.request).then((response) => {
            return response || fetch(e.request).then((response) => {
                return caches.open(CACHE_NAME).then((cache) => {
                    cache.put(e.request, response.clone());
                    return response;
                });
            });
        })
    );
});
