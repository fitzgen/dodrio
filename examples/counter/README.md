# 🦀🕸 `rust-webpack-template`

> **Kickstart your Rust, WebAssembly, and Webpack project!**

This template is designed for creating monorepo-style Web applications with
Rust-generated WebAssembly and Webpack without publishing your wasm to NPM.

* Want to create and publish NPM packages with Rust and WebAssembly? [Check out
  `wasm-pack-template`.](https://github.com/rustwasm/wasm-pack-template)

## 🔋 Batteries Included

This template comes pre-configured with all the boilerplate for compiling Rust
to WebAssembly and hooking into a Webpack build pipeline.

* `npm run start` -- Serve the project locally for development at
  `http://localhost:8080`.

* `npm run build` -- Bundle the project (in production mode).

## 🚴 Using This Template

First, [install `wasm-pack`!](https://rustwasm.github.io/wasm-pack/installer/)

Then, use `npm init` to clone this template:

```sh
npm init rust-webpack my-app
```
