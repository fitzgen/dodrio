# `dodrio`

`dodrio` is an **experimental** virtual DOM library for Rust and WebAssembly. It
is a proving ground for a bump allocation-based virtual DOM architecture, that I
believe is the best way to take advantage of WebAssembly's strengths in the
context of a virtual DOM library.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [Warning](#warning)
- [Design](#design)
  - [Bump Allocation](#bump-allocation)
  - [Change List as Stack Machine](#change-list-as-stack-machine)
  - [Library — Not Framework](#library--not-framework)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Warning

I reiterate that `dodrio` is in a very **experimental** state. It has not
actually been profiled or tuned for performance yet, so while I think the design
should yield a very fast virtual DOM library, `dodrio` is almost certainly not
fast right now. Additionally, it is probably riddled with bugs, and is assuredly
missing features that are critical for actually building Web applications.

## Design

### Bump Allocation

Bump allocation is essentially the fastest method of allocating objects. It has
constraints, but works particularly well when allocation lifetimes match program
phases. And virtual DOMs are very phase oriented.

`dodrio` maintains three bump allocation arenas:

1. The newest, most up-to-date virtual DOM. The virtual DOM nodes themselves and
   any temporary containers needed while creating them are allocated into this
   arena.
2. The previous virtual DOM. This reflects the current state of the physical
   DOM.
3. The difference between (1) and (2). This is a sequence of DOM mutation
   operations — colloquially known as a "change list" — which if applied to
   the physical DOM, will make the physical DOM match (1).

Rendering happens as follows:

* The application state is rendered into bump allocation arena (1).
* (1) diffed with (2) to produce (3).
* JavaScript code applies (3) to the physical DOM.
* (1) and (2) are swapped, double-buffering style, and the new (1) has its bump
  allocation pointer reset.
* Rinse and repeat.

### Change List as Stack Machine

The change list that represents the difference between how the physical DOM
currently looks, and our ideal virtual DOM state is encoded in a tiny stack
machine language. A stack machine works particularly well for applying DOM
diffs, a task that is essentially a tree traversal.

### Library — Not Framework

`dodrio` is just a library. (And did I mention it is experimental?!) It is not a
full-fledged, complete, batteries-included solution for all frontend Web
development. And it never intends to become that either. Its highest ambition is
to prove that its bump allocation-based design is a good one, and maaaayyyyyybe
become a production-grade virtual DOM library that you could plug into a larger
application or toolkit eventually. But it will never be a complete,
batteries-included framework.
