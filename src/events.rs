use crate::{node::Node, vdom::VdomInner, Listener};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

cfg_if::cfg_if! {
    if #[cfg(all(feature = "xxx-unstable-internal-use-only", not(target_arch = "wasm32")))] {
        #[derive(Debug)]
        pub(crate) struct EventsRegistry {}
        impl EventsRegistry {
            pub(crate) fn new(_vdom: Weak<VdomInner>) -> (
                Rc<RefCell<EventsRegistry>>,
                crate::EventsTrampoline,
            ) {
                (Rc::new(RefCell::new(EventsRegistry {})), ())
            }
            pub(crate) fn remove(&mut self, _listener: &Listener) {}
            pub(crate) fn remove_subtree(&mut self, _node: &Node) {}
            pub(crate) unsafe fn add<'a>(&mut self, _listener: &'a Listener<'a>) {}
            pub(crate) fn clear_active_listeners(&mut self) {}
        }
    } else {
        use crate::{
            node::{ElementNode, ListenerCallback, NodeKind},
            vdom::VdomWeak,
        };
        use fxhash::FxHashMap;
        use std::fmt;
        use std::mem;
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::prelude::*;

        /// The events registry manages event listeners for a virtual DOM.
        ///
        /// The events registry is persistent across virtual DOM rendering and double
        /// buffering.
        pub(crate) struct EventsRegistry {
            vdom: Weak<VdomInner>,
            active: FxHashMap<(u32, u32), ListenerCallback<'static>>,
        }

        impl fmt::Debug for EventsRegistry {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.debug_struct("EventsRegistry")
                    .field("active", &self.active.keys().collect::<Vec<_>>())
                    .finish()
            }
        }

        impl EventsRegistry {
            /// Construct a new events registry and JS function trampoline that weakly
            /// holds the new registry and can be used by JS to invoke listeners on the
            /// Rust side.
            pub(crate) fn new(
                vdom: Weak<VdomInner>,
            ) -> (
                Rc<RefCell<EventsRegistry>>,
                crate::EventsTrampoline,
            ) {
                let registry = Rc::new(RefCell::new(EventsRegistry {
                    vdom,
                    active: FxHashMap::default(),
                }));

                let weak_registry = Rc::downgrade(&registry);
                let closure = Closure::wrap(Box::new(move |event, a, b| {
                    debug_assert!(a != 0);

                    // if the VdomInnerExclusive is keeping this closure alive, then the
                    // VdomInnerExclusive should also be keeping the registry alive
                    let registry = weak_registry.upgrade().unwrap_throw();
                    let registry = registry.borrow();

                    match registry.active.get(&(a, b)) {
                        None => warn!(
                            "EventsRegistry closure invoked with unknown listener parts: \
                             (0x{:x}, 0x{:x})",
                            a, b
                        ),
                        Some(callback) => {
                            let vdom = registry.vdom.upgrade().expect_throw(
                                "if the registry is still around, then the vdom should still be around",
                            );
                            let vdom_weak = VdomWeak::new(&vdom);
                            let mut vdom = vdom.exclusive.borrow_mut();
                            let component = vdom.component_raw_mut();
                            callback(component, vdom_weak, event);
                        }
                    }
                }) as Box<dyn Fn(web_sys::Event, u32, u32)>);

                (registry, closure)
            }

            pub(crate) fn remove(&mut self, listener: &Listener) {
                let id = listener.get_callback_parts();
                debug_assert!(id.0 != 0);
                self.active.remove(&id);
            }

            pub(crate) fn remove_subtree(&mut self, node: &Node) {
                match node.kind {
                    NodeKind::Cached(_) | NodeKind::Text(_) => {},
                    NodeKind::Element(&ElementNode {listeners, children, ..}) => {
                        for l in listeners {
                            self.remove(l);
                        }
                        for child in children {
                            self.remove_subtree(child)
                        }
                    }
                }
            }

            /// Add an event listener to the registry, exposing to JS.
            ///
            /// # Unsafety
            ///
            /// The listener's lifetime is extended to `'static` and it is the
            /// caller's responsibility to ensure that the listener is not kept
            /// in the registry after it is dropped. This is maintained during
            /// diffing.
            pub(crate) unsafe fn add<'a>(&mut self, listener: &'a Listener<'a>) {
                let id = listener.get_callback_parts();
                debug_assert!(id.0 != 0);

                let callback =
                    mem::transmute::<ListenerCallback<'a>, ListenerCallback<'static>>(listener.callback);
                let old = self.active.insert(id, callback);
                debug_assert!(old.is_none());
            }

            /// Clear all event listeners from the registry.
            pub(crate) fn clear_active_listeners(&mut self) {
                self.active.clear();
            }
        }
    }
}
