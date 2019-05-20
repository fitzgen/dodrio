# Dodrio

A fast, bump-allocated virtual DOM library for Rust and WebAssembly. Note that
Dodrio is still **experimental**.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [Warning](#warning)
- [Examples](#examples)
- [Design](#design)
  - [Bump Allocation](#bump-allocation)
  - [Change List as Stack Machine](#change-list-as-stack-machine)
  - [Library — Not Framework](#library--not-framework)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Warning

I reiterate that Dodrio is in a very **experimental** state. It probably has
bugs, and no one is using it in production.

## Examples

Here is the classic "Hello, World!" example:

```rust
struct Hello {
    who: String,
}

impl Render for Hello {
    fn render<'a>(&self, cx: &mut RenderContext<a>) -> Node<'a> {
        let who = bumpalo::format!(in cx.bump, "Hello, {}!", self.who);
        div(cx)
            .children([text(who.into_bump_str())])
            .finish()
    }
}
```

More examples can be found in [the `examples`
directory](https://github.com/fitzgen/dodrio/tree/master/examples), including:

* [`counter`](https://github.com/fitzgen/dodrio/tree/master/examples/counter):
  Incrementing and decrementing a counter.
* [`input-form`](https://github.com/fitzgen/dodrio/tree/master/examples/input-form):
  Reading an `<input>` and displaying its contents.
* [`todomvc`](https://github.com/fitzgen/dodrio/tree/master/examples/todomvc):
  An implementation of the infamous TodoMVC application.
* [`moire`](https://github.com/fitzgen/dodrio/tree/master/examples/moire): The
  WebRender Moiré patterns demo.
* [`game-of-life`](https://github.com/fitzgen/dodrio/tree/master/examples/game-of-life):
  The Rust and WebAssembly book's Game of Life tutorial rendered with Dodrio
  instead of to 2D canvas.
* [`js-component`](https://github.com/fitzgen/dodrio/tree/master/examples/js-component):
  Defines a rendering component in JavaScript with the `dodrio-js-api` crate.

## Cargo Features

* `log` &mdash; enable debugging-oriented log messages with the `log` crate's
  facade. You still have to initialize a logger for the messages to go anywhere,
  such as [`console_log`](https://github.com/iamcodemaker/console_log).

* `serde` &mdash; enable `serde::{Serialize, Deserialize}` implementations for
  `Cached<R>` where `R` is serializable and deserializable.

## Design

### Bump Allocation

Bump allocation is essentially the fastest method of allocating objects. It has
constraints, but works particularly well when allocation lifetimes match program
phases. And virtual DOMs are very phase oriented.

Dodrio maintains three bump allocation arenas:

1. The newest, most up-to-date virtual DOM. The virtual DOM nodes themselves and
   any temporary containers needed while creating them are allocated into this
   arena.
2. The previous virtual DOM. This reflects the current state of the physical
   DOM.
3. The difference between (1) and (2). This is a sequence of DOM mutation
   operations — colloquially known as a "change list" — which if applied to
   the physical DOM, will make the physical DOM match (1).

Rendering happens as follows:

1. The application state is rendered into bump allocation arena (1).
2. (1) is diffed with (2) and the changes are emitted into (3).
3. JavaScript code applies the change list in (3) to the physical DOM.
4. (1) and (2) are swapped, double-buffering style, and the new (1) has its bump
   allocation pointer reset, as does (3).
5. Rinse and repeat.

### Change List as Stack Machine

The change list that represents the difference between how the physical DOM
currently looks, and our ideal virtual DOM state is encoded in a tiny stack
machine language. A stack machine works particularly well for applying DOM
diffs, a task that is essentially a tree traversal.

### Library — Not Framework

Dodrio is just a library. (And did I mention it is experimental?!) It is not a
full-fledged, complete, batteries-included solution for all frontend Web
development. And it never intends to become that either.
