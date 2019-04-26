# Sierpinski Triangle

A Sierpinkski triangle that constantly shrinks and grows, and whose nodes have
a value that increments by one every second.

Ported from the React Fiber example [here](https://github.com/claudiopro/react-fiber-vs-stack-demo).

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
