const CACHE_NAME = 'xiangqi-pwa-v2';
const REPO_NAME = '/GameCoTuong';
const ASSETS = [
    './',
    './index.html',
    './manifest.json',
    './icon.png',
    // Trunk generated files will need to be cached dynamically or via a build step that injects them.
    // For now, we rely on the browser cache for those or the runtime caching below.
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
