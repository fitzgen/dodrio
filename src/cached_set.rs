use crate::{
    events::EventsRegistry,
    node::{Node, NodeKind},
    render_context::RenderContext,
};
use bumpalo::Bump;
use fxhash::{FxHashMap, FxHashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::u32;
use wasm_bindgen::prelude::*;

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub_unstable_internal! {
    #[derive(Debug, Default)]
    pub(crate) struct CachedSet {
        items: FxHashMap<CacheId, CacheEntry>,
    }
}

pub_unstable_internal! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
    pub(crate) struct CacheId(u32);
}

#[derive(Debug)]
pub(crate) struct CacheEntry {
    bump: Bump,

    // Actually a reference into `bump`.
    node: *const Node<'static>,

    // All the other cached entries that `node` references (including transitive
    // edges). By eagerly following transitive edges and recording them the
    // once, we don't have to repeat the tracing every time we want to garbage
    // collect cache entries.
    edges: FxHashSet<CacheId>,
}

impl CachedSet {
    pub(crate) fn gc(&mut self, registry: &mut EventsRegistry, roots: &[CacheId]) {
        // TODO: We need to port a hash set over in `bumpalo::collections` so
        // that we can use a temporary bump arena for this set.
        let mut mark_bits = FxHashSet::default();
        mark_bits.reserve(self.items.len());

        for root in roots {
            if mark_bits.insert(*root) {
                let entry = self
                    .items
                    .get(root)
                    .expect_throw("CachedSet::gc: should have root in cached set");

                // NB: no need to recurse here because `edges` already contains
                // transitive edges!
                mark_bits.extend(entry.edges.iter().cloned());
            }
        }

        self.items.retain(|id, entry| {
            let keep = mark_bits.contains(id);
            if !keep {
                debug!("CachedSet::gc: removing {:?}", id);
                let node: &Node = unsafe { &*entry.node };
                registry.remove_subtree(node);
            }
            keep
        });
    }

    // Trace all the transitive edges to other cached entries that the given
    // node has.
    fn trace(&self, node: &Node) -> FxHashSet<CacheId> {
        let mut edges = FxHashSet::default();

        let bump = Bump::new();
        let mut stack = bumpalo::collections::Vec::with_capacity_in(64, &bump);
        stack.push(node);

        while let Some(node) = stack.pop() {
            match &node.kind {
                NodeKind::Text(_) => continue,
                NodeKind::Cached(c) => {
                    debug_assert!(self.items.contains_key(&c.id));
                    edges.insert(c.id);
                    edges.extend(
                        self.items
                            .get(&c.id)
                            .expect_throw("CachedSet::trace: should have c.id in cached set")
                            .edges
                            .iter()
                            .cloned(),
                    );
                }
                NodeKind::Element(el) => {
                    stack.extend(el.children);
                }
            }
        }

        edges
    }

    fn next_id(&mut self) -> CacheId {
        let next = ID_COUNTER.fetch_add(1, Ordering::AcqRel) as u32;
        let next = if next == u32::MAX { None } else { Some(next) };
        CacheId(next.expect_throw("ID_COUNTER overflowed"))
    }

    pub(crate) fn insert<F>(cx: &mut RenderContext, f: F) -> CacheId
    where
        F: for<'a> FnOnce(&mut RenderContext<'a>) -> Node<'a>,
    {
        let bump = Bump::new();
        let set = cx.cached_set;
        let mut nested_cx = RenderContext::new(&bump, set);
        let node = f(&mut nested_cx);
        let node = bump.alloc(node);
        let edges = {
            let set = set.borrow();
            set.trace(node)
        };
        let node = node as *mut Node<'_> as usize as *const Node<'static>;
        let entry = CacheEntry { bump, node, edges };

        let mut set = set.borrow_mut();
        let id = set.next_id();
        debug!("CachedSet::insert: id = {:?}; entry = {:?}", id, entry);
        set.items.insert(id, entry);
        id
    }

    /// Does the cached set contain a cached node with the given id?
    pub fn contains(&self, id: CacheId) -> bool {
        self.items.contains_key(&id)
    }

    /// Get the node for the given cache id.
    pub fn get(&self, mut id: CacheId) -> Node {
        debug!("CachedSet::get: id = {:?}", id);
        loop {
            let entry = self
                .items
                .get(&id)
                .expect_throw("CachedSet::get: should have id in set");
            let node: &Node = unsafe { &*entry.node };
            if let NodeKind::Cached(ref c) = node.kind {
                id = c.id;
                continue;
            } else {
                return node.clone();
            }
        }
    }
}
