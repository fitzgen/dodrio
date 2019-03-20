use super::change_list::ChangeList;
use super::node::{Attribute, ElementNode, Listener, Node, TextNode};
use super::RootRender;
use crate::events::EventsRegistry;
use bumpalo::Bump;
use futures::future::Future;
use std::cell::Cell;
use std::cell::RefCell;
use std::cmp;
use std::fmt;
use std::mem;
use std::mem::ManuallyDrop;
use std::rc::{Rc, Weak};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

/// A strong handle to a mounted virtual DOM.
///
/// When this handle is dropped, the virtual DOM is unmounted and its listeners
/// removed. To keep it mounted forever, use the `Vdom::forget` method.
#[must_use = "A `Vdom` will only keep rendering and listening to events while it has not been \
              dropped. If you want a `Vdom` to run forever, call `Vdom::forget`."]
#[derive(Debug)]
pub struct Vdom {
    inner: Rc<VdomInner>,
}

/// A weak handle to a virtual DOM.
///
/// Does not prevent the virtual DOM from being unmounted: only keeping the
/// original `Vdom` alive guarantees that.
///
/// A `VdomWeak` also gives you the capability to scheduling re-rendering (say
/// after mutating the render component state).
#[derive(Clone, Debug)]
pub struct VdomWeak {
    inner: Weak<VdomInner>,
}

#[derive(Debug)]
pub(crate) struct VdomInner {
    pub(crate) shared: VdomInnerShared,
    pub(crate) exclusive: RefCell<VdomInnerExclusive>,
}

pub(crate) struct VdomInnerShared {
    pub(crate) render_scheduled: Cell<Option<js_sys::Promise>>,
}

pub(crate) struct VdomInnerExclusive {
    // Always `Some` except just before we drop. Just an option so that
    // `unmount` can take the component out but we can still have a Drop
    // implementation.
    component: Option<Box<RootRender>>,

    dom_buffers: [Bump; 2],
    change_list: ManuallyDrop<ChangeList>,
    container: crate::Element,
    events_registry: Option<Rc<RefCell<EventsRegistry>>>,

    // Actually a reference into `self.dom_buffers[0]` or if `self.component` is
    // caching renders, into `self.component`'s bump.
    current_root: Option<Node<'static>>,
}

unsafe fn extend_node_lifetime<'a>(node: Node<'a>) -> Node<'static> {
    mem::transmute(node)
}

impl fmt::Debug for VdomInnerShared {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let render_scheduled = Cell::new(None);
        self.render_scheduled.swap(&render_scheduled);
        let render_scheduled = render_scheduled.into_inner();
        let r = f
            .debug_struct("VdomInnerShared")
            .field("render_scheduled", &render_scheduled)
            .finish();
        self.render_scheduled.set(render_scheduled);
        r
    }
}

impl fmt::Debug for VdomInnerExclusive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VdomInnerExclusive")
            .field("component", &"..")
            .field("dom_buffers", &self.dom_buffers)
            .field("change_list", &self.change_list)
            .field("container", &self.container)
            .field("events_registry", &self.events_registry)
            .field("current_root", &self.current_root)
            .finish()
    }
}

impl Drop for VdomInnerExclusive {
    fn drop(&mut self) {
        debug!("Dropping VdomInnerExclusive");

        // Make sure that we clean up our JS listeners and all that before we
        // empty the container.
        unsafe {
            ManuallyDrop::drop(&mut self.change_list);
        }

        let registry = self.events_registry.take().unwrap_throw();
        let mut registry = registry.borrow_mut();
        registry.clear_active_listeners();

        empty_container(&self.container);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "xxx-unstable-internal-use-only")] {
        fn empty_container(_container: &crate::Element) {}
        fn initialize_container(_container: &crate::Element) {}
    } else {
        fn empty_container(container: &crate::Element) {
            container.set_inner_html("");
        }

        fn initialize_container(container: &crate::Element) {
            empty_container(container);

            // Create the dummy `<div/>` child in the container.
            let window = web_sys::window().expect_throw("should have access to the Window");
            let document = window
                .document()
                .expect("should have access to the Document");
            container
                .append_child(
                    document
                        .create_element("div")
                        .expect("should create element OK")
                        .as_ref(),
                )
                .expect("should append child OK");
        }
    }
}

impl Vdom {
    /// Mount a new `Vdom` in the given container element with the given root
    /// rendering component.
    ///
    /// This will box the given component into trait object.
    pub fn new<R>(container: &crate::Element, component: R) -> Vdom
    where
        R: RootRender,
    {
        Self::with_boxed_root_render(container, Box::new(component) as Box<RootRender>)
    }

    /// Construct a `Vdom` with the already-boxed-as-a-trait-object root
    /// rendering component.
    pub fn with_boxed_root_render<'a, 'bump>(
        container: &crate::Element,
        component: Box<RootRender>,
    ) -> Vdom {
        let dom_buffers = [Bump::new(), Bump::new()];
        let change_list = ManuallyDrop::new(ChangeList::new(container));

        // Create a dummy `<div/>` in our container.
        initialize_container(container);
        let current_root = Node::element(&dom_buffers[0], "div", [], [], [], None);
        let current_root = Some(unsafe { extend_node_lifetime(current_root) });

        let container = container.clone();
        let inner = Rc::new(VdomInner {
            shared: VdomInnerShared {
                render_scheduled: Cell::new(None),
            },
            exclusive: RefCell::new(VdomInnerExclusive {
                component: Some(component),
                dom_buffers,
                change_list,
                container,
                current_root,
                events_registry: None,
            }),
        });

        let (events_registry, events_trampoline) = EventsRegistry::new(Rc::downgrade(&inner));

        {
            let mut inner = inner.exclusive.borrow_mut();
            inner.events_registry = Some(events_registry);
            inner.change_list.init_events_trampoline(events_trampoline);

            // Diff and apply the `contents` against our dummy `<div/>`.
            inner.render();
        }

        Vdom { inner }
    }

    /// Immediately re-render and diff. Only for internal testing and
    /// benchmarking purposes.
    #[cfg(feature = "xxx-unstable-internal-use-only")]
    pub fn immediately_render_and_diff<R>(&self, component: R)
    where
        R: RootRender,
    {
        let mut exclusive = self.inner.exclusive.borrow_mut();
        let component = Box::new(component) as Box<RootRender>;
        exclusive.component = Some(component);
        exclusive.render();
    }

    /// Run this virtual DOM and its listeners forever and never unmount it.
    #[inline]
    pub fn forget(self) {
        mem::forget(self);
    }

    /// Get a weak handle to this virtual DOM.
    #[inline]
    pub fn weak(&self) -> VdomWeak {
        VdomWeak::new(&self.inner)
    }

    /// Unmount this virtual DOM, unregister its event listeners, and return its
    /// root render component.
    #[inline]
    pub fn unmount(self) -> Box<RootRender> {
        Rc::try_unwrap(self.inner.clone())
            .map_err(|_| ())
            .unwrap_throw()
            .exclusive
            .into_inner()
            .component
            .take()
            .unwrap_throw()
    }
}

impl VdomInnerExclusive {
    /// Get an exclusive reference to the underlying render component as a raw
    /// trait object.
    #[inline]
    pub(crate) fn component_raw_mut(&mut self) -> &mut dyn RootRender {
        &mut **self.component.as_mut().unwrap_throw()
    }

    /// Re-render this virtual dom's current component.
    pub(crate) fn render(&mut self) {
        unsafe {
            let events_registry = self.events_registry.take().unwrap();

            {
                // All the old listeners are no longer active. We will build a new
                // set of active listeners when diffing.
                //
                // NB: if we end up avoiding diffing cached renders (instead of just
                // avoiding re-rendering them) then we will need to maintain cached
                // active listeners, and can't just clear all active listeners and
                // rebuild them here.
                let mut registry = events_registry.borrow_mut();
                registry.clear_active_listeners();

                // Reset the inactive bump arena's pointer.
                self.dom_buffers[1].reset();

                // Render the new current contents into the inactive bump arena.
                let new_contents = self
                    .component
                    .as_ref()
                    .unwrap_throw()
                    .render(&self.dom_buffers[1]);
                let new_contents = extend_node_lifetime(new_contents);

                // Diff the old contents with the new contents.
                let old_contents = self.current_root.take().unwrap();
                self.diff(&mut registry, old_contents, new_contents.clone());

                // Swap the buffers to make the bump arena with the new contents the
                // active arena, and the old one into the inactive arena.
                self.swap_buffers();
                self.set_current_root(new_contents);
            }

            self.events_registry = Some(events_registry);

            // Find and drop cached strings that aren't in use anymore.
            self.change_list.drop_unused_strings();

            // Tell JS to apply our diff-generated changes to the physical DOM!
            self.change_list.apply_changes();
        }
    }

    fn swap_buffers(&mut self) {
        let (first, second) = self.dom_buffers.as_mut().split_at_mut(1);
        mem::swap(&mut first[0], &mut second[0]);
    }

    unsafe fn set_current_root(&mut self, current: Node<'static>) {
        debug_assert!(self.current_root.is_none());
        self.current_root = Some(current);
    }

    fn diff<'a>(&mut self, registry: &mut EventsRegistry, old: Node<'a>, new: Node<'a>) {
        match (&new, old) {
            (&Node::Text(TextNode { text: new_text }), Node::Text(TextNode { text: old_text })) => {
                debug!("  both are text nodes");
                if new_text != old_text {
                    debug!("  text needs updating");
                    self.change_list.emit_set_text(new_text);
                }
            }
            (&Node::Text(_), Node::Element(_)) => {
                debug!("  replacing a text node with an element");
                self.create(registry, new);
                self.change_list.emit_replace_with();
            }
            (&Node::Element(_), Node::Text(_)) => {
                debug!("  replacing an element with a text node");
                self.create(registry, new);
                self.change_list.emit_replace_with();
            }
            (
                &Node::Element(ElementNode {
                    tag_name: new_tag_name,
                    listeners: new_listeners,
                    attributes: new_attributes,
                    children: new_children,
                    namespace: new_namespace,
                }),
                Node::Element(ElementNode {
                    tag_name: old_tag_name,
                    listeners: old_listeners,
                    attributes: old_attributes,
                    children: old_children,
                    namespace: old_namespace,
                }),
            ) => {
                debug!("  updating an element");
                if new_tag_name != old_tag_name || new_namespace != old_namespace {
                    debug!("  different tag names or namespaces; creating new element and replacing old element");
                    self.create(registry, new);
                    self.change_list.emit_replace_with();
                    return;
                }
                self.diff_listeners(registry, old_listeners, new_listeners);
                self.diff_attributes(old_attributes, new_attributes);
                self.diff_children(registry, old_children, new_children);
            }
        }
    }

    fn diff_listeners<'a>(
        &mut self,
        registry: &mut EventsRegistry,
        old: &'a [Listener<'a>],
        new: &'a [Listener<'a>],
    ) {
        debug!("  updating event listeners");

        'outer1: for new_l in new {
            unsafe {
                // Safety relies on removing `new_l` from the registry manually
                // at the end of its lifetime. This happens when we invoke
                // `clear_active_listeners` at the start of a new rendering
                // phase.
                registry.add(new_l);
            }
            for old_l in old {
                if new_l.event == old_l.event {
                    self.change_list.emit_update_event_listener(new_l);
                    continue 'outer1;
                }
            }
            self.change_list.emit_new_event_listener(new_l);
        }

        'outer2: for old_l in old {
            for new_l in new {
                if new_l.event == old_l.event {
                    continue 'outer2;
                }
            }
            self.change_list.emit_remove_event_listener(old_l.event);
        }
    }

    fn diff_attributes(&mut self, old: &[Attribute], new: &[Attribute]) {
        debug!("  updating attributes");

        // Do O(n^2) passes to add/update and remove attributes, since
        // there are almost always very few attributes.
        'outer: for new_attr in new {
            if new_attr.is_volatile() {
                self.change_list
                    .emit_set_attribute(new_attr.name, new_attr.value);
            } else {
                for old_attr in old {
                    if old_attr.name == new_attr.name {
                        if old_attr.value != new_attr.value {
                            self.change_list
                                .emit_set_attribute(new_attr.name, new_attr.value);
                        }
                        continue 'outer;
                    }
                }
                self.change_list
                    .emit_set_attribute(new_attr.name, new_attr.value);
            }
        }

        'outer2: for old_attr in old {
            for new_attr in new {
                if old_attr.name == new_attr.name {
                    continue 'outer2;
                }
            }
            self.change_list.emit_remove_attribute(old_attr.name);
        }
    }

    fn diff_children<'a>(
        &mut self,
        registry: &mut EventsRegistry,
        old: &'a [Node<'a>],
        new: &'a [Node<'a>],
    ) {
        debug!("  updating children shared by old and new");

        let num_children_to_diff = cmp::min(new.len(), old.len());
        let mut new_children = new.iter();
        let mut old_children = old.iter();
        let mut pushed = false;

        for (i, (new_child, old_child)) in new_children
            .by_ref()
            .zip(old_children.by_ref())
            .take(num_children_to_diff)
            .enumerate()
        {
            if i == 0 {
                self.change_list.emit_push_first_child();
                pushed = true;
            } else {
                debug_assert!(pushed);
                self.change_list.emit_pop_push_next_sibling();
            }

            self.diff(registry, old_child.clone(), new_child.clone());
        }

        if old_children.next().is_some() {
            debug!("  removing extra old children");
            debug_assert!(new_children.next().is_none());
            if !pushed {
                self.change_list.emit_push_first_child();
            } else {
                self.change_list.emit_pop_push_next_sibling();
            }
            self.change_list.emit_remove_self_and_next_siblings();
            pushed = false;
        } else {
            debug!("  creating new children");
            for (i, new_child) in new_children.enumerate() {
                if i == 0 && pushed {
                    self.change_list.emit_pop();
                    pushed = false;
                }
                self.create(registry, new_child.clone());
                self.change_list.emit_append_child();
            }
        }

        debug!("  done updating children");
        if pushed {
            self.change_list.emit_pop();
        }
    }

    fn create<'a>(&mut self, registry: &mut EventsRegistry, node: Node<'a>) {
        match node {
            Node::Text(TextNode { text }) => {
                self.change_list.emit_create_text_node(text);
            }
            Node::Element(ElementNode {
                tag_name,
                listeners,
                attributes,
                children,
                namespace,
            }) => {
                if let Some(namespace) = namespace {
                    self.change_list.emit_create_element_ns(tag_name, namespace);
                } else {
                    self.change_list.emit_create_element(tag_name);
                }
                for l in listeners {
                    unsafe {
                        registry.add(l);
                    }
                    self.change_list.emit_new_event_listener(l);
                }
                for attr in attributes {
                    if namespace.is_none() || attr.name.starts_with("xmlns") {
                        self.change_list.emit_set_attribute(&attr.name, &attr.value);
                    } else {
                        self.change_list
                            .emit_set_attribute_ns(&attr.name, &attr.value);
                    }
                }
                for child in children {
                    self.create(registry, child.clone());
                    self.change_list.emit_append_child();
                }
            }
        }
    }
}

fn request_animation_frame(f: &Closure<FnMut()>) {
    web_sys::window()
        .expect_throw("should have a window")
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect_throw("should register `requestAnimationFrame` OK");
}

fn with_animation_frame<F>(mut f: F)
where
    F: 'static + FnMut(),
{
    let g = Rc::new(RefCell::new(None));
    let h = g.clone();

    let f = Closure::wrap(Box::new(move || {
        *g.borrow_mut() = None;
        f();
    }) as Box<FnMut()>);
    request_animation_frame(&f);

    *h.borrow_mut() = Some(f);
}

/// An operation failed because the virtual DOM was already dropped and
/// unmounted.
#[derive(Debug)]
pub struct VdomDroppedError {}

impl fmt::Display for VdomDroppedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "The virtual DOM was dropped.")
    }
}

impl std::error::Error for VdomDroppedError {}

impl VdomWeak {
    /// Construct a new weak handle to the given virtual DOM.
    #[inline]
    pub(crate) fn new(inner: &Rc<VdomInner>) -> VdomWeak {
        VdomWeak {
            inner: Rc::downgrade(inner),
        }
    }

    /// Replace the root rendering component with the new `root`.
    ///
    /// Returns a future that resolves to the *old* root component.
    pub fn set_component(
        self,
        root: Box<dyn RootRender>,
    ) -> impl Future<Item = Box<dyn RootRender + 'static>, Error = VdomDroppedError> {
        futures::future::ok(self.inner.upgrade())
            .and_then(|inner| inner.ok_or(()))
            .map_err(|_| VdomDroppedError {})
            .and_then(|inner| {
                let promise = js_sys::Promise::resolve(&JsValue::null());
                JsFuture::from(promise)
                    .map_err(|_| VdomDroppedError {})
                    .and_then(move |_| {
                        let old = {
                            let mut exclusive = inner.exclusive.borrow_mut();
                            mem::replace(&mut *exclusive.component.as_mut().unwrap_throw(), root)
                        };
                        VdomWeak::new(&inner).render().map(|_| old)
                    })
            })
    }

    /// Execute `f` with a reference to this virtual DOM's root rendering
    /// component.
    ///
    /// To ensure exclusive access to the root rendering component, the
    /// invocation takes place on a new tick of the micro-task queue.
    pub fn with_component<F, T>(&self, f: F) -> impl Future<Item = T, Error = VdomDroppedError>
    where
        F: 'static + FnOnce(&mut dyn RootRender) -> T,
    {
        futures::future::ok(self.inner.upgrade())
            .and_then(|inner| inner.ok_or(()))
            .map_err(|_| VdomDroppedError {})
            .and_then(|inner| {
                let mut f = Some(f);
                let promise = js_sys::Promise::resolve(&JsValue::null());
                JsFuture::from(promise)
                    .map_err(|_| VdomDroppedError {})
                    .map(move |_| {
                        let f = f.take().unwrap_throw();
                        let mut exclusive = inner.exclusive.borrow_mut();
                        f(exclusive.component_raw_mut())
                    })
            })
    }

    /// Schedule a render to occur during the next animation frame.
    ///
    /// If you want a future that resolves after the render has finished, use
    /// `render` instead.
    pub fn schedule_render(&self) {
        debug!("VdomWeak::schedule_render");
        wasm_bindgen_futures::spawn_local(self.render().map_err(|_| ()));
    }

    /// Schedule a render to occur during the next animation frame and return a
    /// future that will complete once the render has finished.
    ///
    /// If you don't want to do more things after the render completes, then use
    /// `schedule_render` instead of `render`.
    pub fn render(&self) -> impl Future<Item = (), Error = VdomDroppedError> {
        debug!("VdomWeak::render: initiating render in new animation frame");
        futures::future::ok(self.inner.upgrade())
            .and_then(|inner| inner.ok_or(()))
            .map_err(|_| VdomDroppedError {})
            .and_then(|inner| {
                let promise = inner.shared.render_scheduled.take().unwrap_or_else(|| {
                    js_sys::Promise::new(&mut |resolve, reject| {
                        let vdom = VdomWeak {
                            inner: Rc::downgrade(&inner),
                        };
                        with_animation_frame(move || match vdom.inner.upgrade() {
                            None => {
                                warn!("VdomWeak::render: vdom unmounted before we could render");
                                let r = reject.call0(&JsValue::null());
                                debug_assert!(r.is_ok());
                            }
                            Some(inner) => {
                                let mut exclusive = inner.exclusive.borrow_mut();
                                exclusive.render();

                                // We did the render, so take the promise away
                                // and let future `render` calls request new
                                // animation frames.
                                let _ = inner.shared.render_scheduled.take();

                                debug!("VdomWeak::render: finished rendering");
                                let r = resolve.call0(&JsValue::null());
                                debug_assert!(r.is_ok());
                            }
                        });
                    })
                });
                inner.shared.render_scheduled.set(Some(promise.clone()));
                JsFuture::from(promise)
                    .map(|_| ())
                    .map_err(|_| VdomDroppedError {})
            })
    }
}
