# Horizon client comparison slices

Disposable, local-only experiments with fictional in-memory data. Neither is production code, and the two layouts are not yet a controlled or complete comparison. No backend, authentication, notifications, real calendar, or map is included.

With Docker running, from this directory:

```sh
docker compose up --build
```

- React: http://localhost:5173
- Flutter: http://localhost:8080

Stop with Ctrl-C, then `docker compose down`. First builds require network access and can take time. Images use a Flutter `stable` tag and Node 22 rather than fully pinned toolchains; record actual versions before comparing. The apps run development servers, not production builds; do not use them to compare startup performance. For real-device testing, localhost-only port bindings must be deliberately adjusted and the host/network secured.

Before choosing a client, match the slices' layouts and behavior, define performance thresholds, use production builds, and run the device and accessibility checks described in `docs/technology/stack-comparison.md`.

To run the draft behavior tests after rebuilding:

```sh
docker compose run --rm --no-deps react npm test
docker compose run --rm --no-deps flutter flutter test test/comparison_test.dart
```

The React test dependency install reported vulnerabilities; do not expose the experiment publicly or reuse the dependencies for production without review. The runtime-configuration experiment has not been implemented.
