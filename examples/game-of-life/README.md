# Conway's Game of Life

A port of the [Rust and WebAssembly tutorial's Game of Life
implementation](https://rustwasm.github.io/book/game-of-life/introduction.html)
to rendering to HTML with Dodrio instead of to a canvas via JavaScript.

## Source

See `src/lib.rs`.

## Build

```
wasm-pack build --target web
```

## Serve

Use any HTTP server, for example:

```
python -m SimpleHTTPServer
```
