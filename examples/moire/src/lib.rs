mod colors;

use cfg_if::cfg_if;
use dodrio::{bumpalo, Node, Render, RenderContext, Vdom};
use std::cell::RefCell;
use std::rc::Rc;
use std::str;
use wasm_bindgen::{prelude::*, JsCast};

/// This is the main function that is automatically invoked when the wasm module
/// is loaded.
#[wasm_bindgen(start)]
pub fn run() {
    // Set up the panic hook for debugging when things go wrong.
    init_logging();

    // Grab the document's `<body>`.
    let window = web_sys::window().unwrap_throw();
    let document = window.document().unwrap_throw();
    let scene = document.get_element_by_id("scene").unwrap_throw();

    // Create a new `Moire` render component.
    let component = Moire::new();

    // Create a virtual DOM and mount it and the `Moire` render component to the
    // scene.
    let vdom = Vdom::new(&scene, component);

    // Kick off a loop that keeps re-rendering on every animation frame.
    let rc = <Rc<RefCell<Option<Closure<dyn FnMut()>>>>>::default();
    let rc2 = rc.clone();
    let window2 = window.clone();
    let weak = vdom.weak();
    let f = Closure::wrap(Box::new(move || {
        weak.schedule_render();
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

cfg_if! {
    if #[cfg(feature = "logging")] {
        fn init_logging() {
            console_error_panic_hook::set_once();
            console_log::init_with_level(log::Level::Trace)
                .expect_throw("should initialize logging OK");
        }
    } else {
        fn init_logging() {
            // Do nothing.
        }
    }
}

/// A rendering component that renders two overlapping sets of concentric
/// circles that move around and form Moiré patterns.
///
/// https://en.wikipedia.org/wiki/Moir%C3%A9_pattern
pub struct Moire {
    // How many circles to render in each of our sets. This is controlled by the
    // `<input>` slider.
    count: u32,
}

impl Moire {
    /// Construct a new `Moire` rendering component.
    ///
    /// Defaults to 200 circles.
    pub fn new() -> Moire {
        Moire { count: 200 }
    }

    /// Callback for "change" events emitted on the `<input>` slider that
    /// controls how many circles we render on each frame.
    fn on_change(&mut self, event: web_sys::Event) {
        // Get the `<input>` element.
        let input = event
            .target()
            .unwrap_throw()
            .unchecked_into::<web_sys::HtmlInputElement>();

        // Parse its new value.
        let value = input.value();
        let value: u32 = value
            .parse()
            .expect_throw("<input type=\"range\"> value should always be an integer");

        // And update our circle count!
        self.count = value;
    }
}

impl<'a> Render<'a> for Moire {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;

        let elapsed = web_sys::window()
            .unwrap_throw()
            .performance()
            .unwrap_throw()
            .now();
        let elapsed = elapsed / 1600.0;

        main(&cx)
            .attr("style", self.moire_style(cx, elapsed))
            .children([
                // The `<input>` that lets users control how many circles to
                // render.
                input(&cx)
                    .attr("id", "circle-count")
                    .attr("type", "range")
                    .attr("min", "30")
                    .attr("max", "500")
                    .attr("value", {
                        let count = bumpalo::format!(in cx.bump, "{}", self.count);
                        count.into_bump_str()
                    })
                    .on("change", |root, _vdom, event| {
                        root.unwrap_mut::<Moire>().on_change(event);

                        // Note: there is no need to manually schedule a
                        // re-render here, since we already started a
                        // `requestAnimationFrame` loop that automatically
                        // re-renders on every frame in the `run` function.
                    })
                    .finish(),
                // And the actual concentric circles that form the Moire patterns.
                self.orbiting_objects(cx, elapsed),
                self.lemniscate_objects(cx, elapsed),
            ])
            .finish()
    }
}

/// Rendering helper methods.
impl Moire {
    /// Generate the main Moire element's inline CSS styles.
    fn moire_style<'a>(&self, cx: &mut RenderContext<'a>, elapsed: f64) -> &'a str {
        let elapsed = elapsed / 3.0;
        let color = colors::get_interpolated_color(|(_, bg)| bg, elapsed % 1.0);
        let style = bumpalo::format!(
            in cx.bump,
            "background-color: rgb({}, {}, {})",
            color.r,
            color.g,
            color.b
        );
        style.into_bump_str()
    }

    /// Generate the orbiting circle objects.
    fn orbiting_objects<'a>(&self, cx: &mut RenderContext<'a>, elapsed: f64) -> Node<'a> {
        const D: f64 = 200.0;
        let x = elapsed.sin() * D;
        let y = elapsed.cos() * D;
        self.moving_object(cx, elapsed, x, y)
    }

    /// Generate the lemniscate circle objects that interact with the orbiting
    /// circle objects to form Moire patterns.
    fn lemniscate_objects<'a>(&self, cx: &mut RenderContext<'a>, elapsed: f64) -> Node<'a> {
        const A: f64 = 150.0;
        let x = elapsed.sin() * A;
        let y = elapsed.sin() * elapsed.cos() * A;
        self.moving_object(cx, elapsed, x, y)
    }

    fn moving_object<'a>(
        &self,
        cx: &mut RenderContext<'a>,
        elapsed: f64,
        x: f64,
        y: f64,
    ) -> Node<'a> {
        use dodrio::builder::*;
        div(&cx)
            .attr("class", "object")
            .attr("style", self.moving_object_style(cx, x, y))
            .children([self.circle(cx, elapsed, self.count)])
            .finish()
    }

    /// Generate inline CSS styles for a moving object.
    fn moving_object_style<'a>(&self, cx: &mut RenderContext<'a>, x: f64, y: f64) -> &'a str {
        bumpalo::format!(in cx.bump, "left: {}px; top: {}px;", x, y).into_bump_str()
    }

    /// Recursively generate `self.count` concentric circles.
    fn circle<'a>(&self, cx: &mut RenderContext<'a>, elapsed: f64, n: u32) -> Node<'a> {
        use dodrio::builder::*;

        let r = n * 16;

        let mut circle = div(&cx)
            .attr("class", "circle")
            .attr("data-radius", {
                let r = bumpalo::format!(in cx.bump, "{}", r);
                r.into_bump_str()
            })
            .attr("style", self.circle_style(cx, elapsed, n, r));

        if n > 0 {
            circle = circle.child(self.circle(cx, elapsed, n - 1));
        }

        circle.finish()
    }

    /// Generate inline CSS styles for a circle.
    fn circle_style<'a>(
        &self,
        cx: &mut RenderContext<'a>,
        elapsed: f64,
        n: u32,
        r: u32,
    ) -> &'a str {
        let (border, alpha) = self.circle_color(elapsed, n);
        let margin = r / 2 + 3;
        let style = bumpalo::format!(
            in cx.bump,
            "border-color: rgba({}, {}, {}, {});\
             margin-left: -{}px;\
             margin-top: -{}px;\
             width: {}px;\
             height: {}px;",
            border.r,
            border.g,
            border.b,
            alpha,
            margin,
            margin,
            r,
            r
        );
        style.into_bump_str()
    }

    /// Compute the color for a circle.
    fn circle_color(&self, elapsed: f64, n: u32) -> (colors::Rgb, f64) {
        let elapsed = elapsed / 3.0;
        let base = colors::get_interpolated_color(|(fg, _)| fg, elapsed % 1.0);
        let lightness = 1.0 - (n as f64) / (self.count as f64);
        (base, lightness)
    }
}
