# Moiré Patterns!

This was originally a demo meant to show off [WebRender][], and so the
performance bottleneck is painting, not DOM manipulation, but it looks cool
either way!

[WebRender]: https://hacks.mozilla.org/2017/10/the-whole-web-at-maximum-fps-how-webrender-gets-rid-of-jank/

## Source

See `src/lib.rs` for the main bits, and `src/colors.rs` for HSV to RGB color
conversion and other color utilities.

## Build

```
wasm-pack build --target web
```

## Serve

Use any HTTP server, for example:

```
python -m SimpleHTTPServer
```
