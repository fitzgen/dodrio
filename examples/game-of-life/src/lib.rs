use dodrio::{bumpalo, Node, Render, RenderContext, Vdom};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{prelude::*, JsCast};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Dead = 0,
    Alive = 1,
}

/// The universe containing all of the cells for our Game of Life.
struct Universe {
    width: u32,
    height: u32,
    cells: Vec<Cell>,
}

impl Universe {
    /// Construct a new universe.
    pub fn new() -> Universe {
        let width = 64;
        let height = 64;

        let cells = (0..width * height)
            .map(|i| {
                if i % 2 == 0 || i % 7 == 0 {
                    Cell::Alive
                } else {
                    Cell::Dead
                }
            })
            .collect();

        Universe {
            width,
            height,
            cells,
        }
    }

    /// Compute one tick of the universe.
    pub fn tick(&mut self) {
        let mut next = self.cells.clone();

        for row in 0..self.height {
            for col in 0..self.width {
                let idx = self.get_index(row, col);
                let cell = self.cells[idx];
                let live_neighbors = self.live_neighbor_count(row, col);

                let next_cell = match (cell, live_neighbors) {
                    // Rule 1: Any live cell with fewer than two live neighbours
                    // dies, as if caused by underpopulation.
                    (Cell::Alive, x) if x < 2 => Cell::Dead,
                    // Rule 2: Any live cell with two or three live neighbours
                    // lives on to the next generation.
                    (Cell::Alive, 2) | (Cell::Alive, 3) => Cell::Alive,
                    // Rule 3: Any live cell with more than three live
                    // neighbours dies, as if by overpopulation.
                    (Cell::Alive, x) if x > 3 => Cell::Dead,
                    // Rule 4: Any dead cell with exactly three live neighbours
                    // becomes a live cell, as if by reproduction.
                    (Cell::Dead, 3) => Cell::Alive,
                    // All other cells remain in the same state.
                    (otherwise, _) => otherwise,
                };

                next[idx] = next_cell;
            }
        }

        self.cells = next;
    }

    fn get_index(&self, row: u32, column: u32) -> usize {
        (row * self.width + column) as usize
    }

    fn live_neighbor_count(&self, row: u32, column: u32) -> u8 {
        let mut count = 0;

        let north = if row == 0 { self.height - 1 } else { row - 1 };

        let south = if row == self.height - 1 { 0 } else { row + 1 };

        let west = if column == 0 {
            self.width - 1
        } else {
            column - 1
        };

        let east = if column == self.width - 1 {
            0
        } else {
            column + 1
        };

        let nw = self.get_index(north, west);
        count += self.cells[nw] as u8;

        let n = self.get_index(north, column);
        count += self.cells[n] as u8;

        let ne = self.get_index(north, east);
        count += self.cells[ne] as u8;

        let w = self.get_index(row, west);
        count += self.cells[w] as u8;

        let e = self.get_index(row, east);
        count += self.cells[e] as u8;

        let sw = self.get_index(south, west);
        count += self.cells[sw] as u8;

        let s = self.get_index(south, column);
        count += self.cells[s] as u8;

        let se = self.get_index(south, east);
        count += self.cells[se] as u8;

        count
    }
}

/// The rendering implementation for our Game of Life.
impl<'a> Render<'a> for Universe {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;

        let mut rows = bumpalo::collections::Vec::with_capacity_in(self.height as usize, cx.bump);

        for row in self.cells.chunks(self.width as usize) {
            let mut cells =
                bumpalo::collections::Vec::with_capacity_in(self.width as usize, cx.bump);

            for cell in row {
                cells.push(
                    span(&cx)
                        .attr("class", "cell")
                        .attr(
                            "style",
                            match cell {
                                Cell::Alive => "background-color: black",
                                Cell::Dead => "background-color: white",
                            },
                        )
                        .finish(),
                );
            }

            rows.push(div(&cx).attr("class", "row").children(cells).finish());
        }

        div(&cx).attr("class", "universe").children(rows).finish()
    }
}

#[wasm_bindgen(start)]
pub fn run() {
    // Set up the panic hook for debugging when things go wrong.
    console_error_panic_hook::set_once();

    // Grab the document's `<body>`.
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let body = document.body().unwrap_throw();

    // Create a new Game of Life `Universe` render component.
    let universe = Universe::new();

    // Create a virtual DOM and mount it and the `Universe` render component to the
    // `<body>`.
    let vdom = Vdom::new(body.as_ref(), universe);

    // Kick off a loop that keeps computing a tick of the universe and then
    // re-rendering on every animation frame.
    let rc = <Rc<RefCell<Option<Closure<dyn FnMut()>>>>>::default();
    let rc2 = rc.clone();
    let window2 = window.clone();
    let weak = vdom.weak();
    let f = Closure::wrap(Box::new(move || {
        weak.schedule_render();

        wasm_bindgen_futures::spawn_local({
            let weak = weak.clone();
            async move {
                let fut = weak.with_component(|root| {
                    let universe = root.unwrap_mut::<Universe>();
                    universe.tick();
                });

                if fut.await.is_err() {
                    wasm_bindgen::throw_str("impossible, we always keep the vdom alive");
                }
            }
        });

        window
            .request_animation_frame(
                rc.borrow()
                    .as_ref()
                    .unwrap_throw()
                    .as_ref()
                    .unchecked_ref::<js_sys::Function>(),
            )
            .unwrap_throw();
    }) as Box<dyn FnMut()>);
    window2
        .request_animation_frame(f.as_ref().unchecked_ref::<js_sys::Function>())
        .unwrap_throw();
    *rc2.borrow_mut() = Some(f);

    // Run the virtual DOM forever and don't unmount it.
    vdom.forget();
}
