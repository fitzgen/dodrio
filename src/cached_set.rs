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

    // The template for this cached virtual subtree.
    //
    // When a cache entry has a template, that means that the `ChangeList` will
    // lazily build a physical DOM subtree based on the template, and when we
    // create new versions of this cached result, we clone the template's
    // physical DOM subtree and then modify it, rather than building the cached
    // result up from scratch. This is a nice win because we bounce between JS
    // and DOM methods less often, and get the C++ DOM implementation to do most
    // of the subtree construction for us.
    template: Option<CacheId>,

    // Whether this entry should never be garbage collected. Typically only
    // templates are pinned.
    pinned: bool,
}

impl From<CacheId> for u32 {
    #[inline]
    fn from(id: CacheId) -> u32 {
        id.0
    }
}

impl CachedSet {
    pub(crate) fn new_roots_set(&self) -> FxHashSet<CacheId> {
        let mut roots = FxHashSet::default();
        roots.reserve(self.items.len());
        roots
    }

    pub(crate) fn gc(&mut self, registry: &mut EventsRegistry, roots: FxHashSet<CacheId>) {
        let mut marked = FxHashSet::default();
        marked.reserve(self.items.len());

        for root in roots {
            if marked.insert(root) {
                let entry = self
                    .items
                    .get(&root)
                    .expect_throw("CachedSet::gc: should have root in cached set");

                // NB: no need to recurse here because `edges` already contains
                // transitive edges!
                marked.extend(entry.edges.iter().cloned());
            }
        }

        self.items.retain(|id, entry| {
            let keep = entry.pinned || marked.contains(id);
            if !keep {
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
        self.trace_recursive(&mut edges, node);
        edges
    }

    fn trace_recursive(&self, edges: &mut FxHashSet<CacheId>, node: &Node) {
        match &node.kind {
            NodeKind::Text(_) => return,
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
                for child in el.children {
                    self.trace_recursive(edges, child);
                }
            }
        }
    }

    fn next_id(&mut self) -> CacheId {
        let next = ID_COUNTER.fetch_add(1, Ordering::AcqRel) as u32;
        let next = if next == u32::MAX { None } else { Some(next) };
        CacheId(next.expect_throw("ID_COUNTER overflowed"))
    }

    pub(crate) fn insert<F>(
        cx: &mut RenderContext,
        pinned: bool,
        template: Option<CacheId>,
        f: F,
    ) -> CacheId
    where
        F: for<'a> FnOnce(&mut RenderContext<'a>) -> Node<'a>,
    {
        let set = cx.cached_set;
        let bump = Bump::new();
        let (node, edges) = {
            let mut nested_cx = RenderContext::new(&bump, cx.cached_set, cx.templates);
            let node = f(&mut nested_cx);
            let node = bump.alloc(node);
            let edges = {
                let set = set.borrow();
                set.trace(node)
            };
            (
                node as *mut Node<'_> as usize as *const Node<'static>,
                edges,
            )
        };

        let entry = CacheEntry {
            bump,
            node,
            edges,
            template,
            pinned,
        };

        let mut set = set.borrow_mut();
        let id = set.next_id();
        set.items.insert(id, entry);
        id
    }

    /// Does the cached set contain a cached node with the given id?
    pub fn contains(&self, id: CacheId) -> bool {
        self.items.contains_key(&id)
    }

    /// Get the cached node and its template (if any) for the given cache id.
    pub fn get(&self, id: CacheId) -> (&Node, Option<CacheId>) {
        let entry = self
            .items
            .get(&id)
            .expect_throw("CachedSet::get: should have id in set");
        let node: &Node = unsafe { &*entry.node };
        (node, entry.template)
    }
}
