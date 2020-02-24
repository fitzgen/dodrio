use super::create_element;
use dodrio::{Node, Render, RenderContext, Vdom};
use futures::future::{select, Either};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

struct EventContainer {
    event: &'static str,
    on_event: Box<dyn FnMut()>,
}

impl EventContainer {
    fn new<F>(event: &'static str, on_event: F) -> EventContainer
    where
        F: 'static + FnMut(),
    {
        EventContainer {
            event,
            on_event: Box::new(on_event),
        }
    }
}

impl<'a> Render<'a> for EventContainer {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;
        div(&cx)
            .attr("id", "target")
            .on(self.event, |root, _scheduler, _event| {
                (root.unwrap_mut::<EventContainer>().on_event)();
            })
            .finish()
    }
}

fn target(container: &web_sys::Element) -> web_sys::HtmlElement {
    container
        .query_selector("#target")
        .expect_throw("should querySelector OK")
        .expect_throw("should find `#target` in container")
        .unchecked_into()
}

#[wasm_bindgen_test]
async fn click() {
    let container = create_element("div");

    let (sender, receiver) = futures::channel::oneshot::channel();
    let mut sender = Some(sender);

    let _vdom = Vdom::new(
        &container,
        EventContainer::new("click", move || {
            sender
                .take()
                .expect_throw("should only call listener once")
                .send(())
                .expect_throw("should not have dropped the receiver");
        }),
    );

    target(&container).click();

    receiver.await.unwrap();
}

#[wasm_bindgen_test]
async fn updated_listener_is_called() {
    let container = create_element("div");

    let (first_sender, first_receiver) = futures::channel::oneshot::channel();
    let mut first_sender = Some(first_sender);

    let vdom = Vdom::new(
        &container,
        EventContainer::new("click", move || {
            first_sender
                .take()
                .expect_throw("should only call listener once")
                .send("first")
                .expect_throw("should not have dropped the receiver");
        }),
    );

    let (second_sender, second_receiver) = futures::channel::oneshot::channel();
    let mut second_sender = Some(second_sender);

    vdom.weak()
        .set_component(Box::new(EventContainer::new("click", move || {
            second_sender
                .take()
                .expect_throw("should only call listener once")
                .send("second")
                .expect_throw("should not have dropped the receiver");
        })))
        .await
        .unwrap();

    target(&container).click();

    match select(first_receiver, second_receiver).await {
        Either::Left((Ok(_), _)) => panic!(),
        Either::Left((Err(_), second)) => assert_eq!(second.await, Ok("second")),
        Either::Right((Ok(msg), _)) => assert_eq!(msg, "second"),
        Either::Right((Err(_), _)) => panic!(),
    }
}

struct ListensOnlyOnFirstRender {
    count: Cell<usize>,
    callback: Box<dyn FnMut()>,
}

impl ListensOnlyOnFirstRender {
    fn new<F>(callback: F) -> ListensOnlyOnFirstRender
    where
        F: 'static + FnMut(),
    {
        ListensOnlyOnFirstRender {
            count: Cell::new(0),
            callback: Box::new(callback),
        }
    }
}

impl<'a> Render<'a> for ListensOnlyOnFirstRender {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::builder::*;

        let count = self.count.get();
        self.count.set(count + 1);

        let mut elem = div(&cx).attr("id", "target");
        if count == 0 {
            elem = elem.on("click", |root, _scheduler, _event| {
                (root.unwrap_mut::<ListensOnlyOnFirstRender>().callback)();
            });
        }
        elem.finish()
    }
}

#[wasm_bindgen_test]
async fn removed_listener_is_not_called() {
    let container = create_element("div");

    let (outer_sender, outer_receiver) = futures::channel::oneshot::channel();
    let outer_sender = Rc::new(RefCell::new(Some(outer_sender)));
    let outer_listener = Closure::wrap(Box::new(move |_| {
        outer_sender
            .borrow_mut()
            .take()
            .expect_throw("should only invoke outer_listener once")
            .send("outer")
            .expect_throw("should not have dropped receiver");
    }) as Box<dyn FnMut(web_sys::Event)>);

    container
        .add_event_listener_with_callback("click", outer_listener.as_ref().unchecked_ref())
        .unwrap();

    let (vdom_sender, vdom_receiver) = futures::channel::oneshot::channel();
    let mut vdom_sender = Some(vdom_sender);

    // Render a vdom with a listener into container.
    let vdom = Vdom::new(
        &container,
        ListensOnlyOnFirstRender::new(move || {
            vdom_sender
                .take()
                .expect_throw("should not invoke vdom listener more than once")
                .send("inner")
                .expect_throw("should not have dropped the receiver");
        }),
    );

    // Re-render, so we aren't listening anymore.
    vdom.weak().render().await.unwrap();

    target(&container).click();

    // We should get our container's event handler fired, and not the unmounted
    // vdom's event handler.
    match select(outer_receiver, vdom_receiver).await {
        Either::Left((Ok(msg), _)) => assert_eq!(msg, ("outer")),
        Either::Left((Err(_), _)) => panic!(),
        Either::Right((Ok(_), _)) => panic!(),
        Either::Right((Err(_), outer)) => assert_eq!(outer.await, Ok("outer")),
    }
}

#[wasm_bindgen_test]
async fn event_does_not_fire_after_unmounting() {
    let container = create_element("div");

    let (outer_sender, outer_receiver) = futures::channel::oneshot::channel();
    let outer_sender = Rc::new(RefCell::new(Some(outer_sender)));
    let outer_listener = Closure::wrap(Box::new(move |_| {
        outer_sender
            .borrow_mut()
            .take()
            .expect_throw("should only invoke outer_listener once")
            .send("outer")
            .expect_throw("should not have dropped receiver");
    }) as Box<dyn FnMut(web_sys::Event)>);

    container
        .add_event_listener_with_callback("click", outer_listener.as_ref().unchecked_ref())
        .unwrap();

    let (vdom_sender, vdom_receiver) = futures::channel::oneshot::channel();
    let mut vdom_sender = Some(vdom_sender);

    // Render a vdom with a listener into container.
    let vdom = Vdom::new(
        &container,
        EventContainer::new("click", move || {
            vdom_sender
                .take()
                .expect_throw("should not invoke vdom listener more than once")
                .send("vdom")
                .expect_throw("should not have dropped the receiver");
        }),
    );

    // Grab a reference to the target before dropping clears the DOM.
    let target = target(&container);

    // Unmount the vdom.
    drop(vdom);

    // The target is no longer a child of the container, so re-append it so that
    // the click will bubble.
    container.append_child(&target).unwrap();

    // Send a click to the target.
    target.click();

    // We should get our container's event handler fired, and not the unmounted
    // vdom's event handler.
    match select(outer_receiver, vdom_receiver).await {
        Either::Left((Ok(msg), _)) => assert_eq!(msg, ("outer")),
        Either::Left((Err(_), _)) => panic!(),
        Either::Right((Ok(_), _)) => panic!(),
        Either::Right((Err(_), outer)) => assert_eq!(outer.await, Ok("outer")),
    }
}
