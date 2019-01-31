use crate::{Node, Render};
use bumpalo::Bump;
use std::cell::RefCell;
use std::mem;
use std::ops::{Deref, DerefMut};

// TODO: This implementation will only cache the rendering (generation of the
// virtual DOM) but not the diffing of the cached subtree. We could skip diffing
// for cached tree by adding `fn is_cached(&self) -> bool` to `Node` that we can
// check during diffing. This comes at the cost of bloating `Node`'s size (since
// we don't have any padding to sneak an extra `bool` field into). We should
// investigate whether it is worth adding this or not.

/// A renderable that supports caching for when rendering is expensive but can
/// generate the same DOM tree.
pub struct Cached<R, Root: 'static> {
    inner: R,
    bump: bumpalo::Bump,
    // Actually a self-reference into `self.bump`. Safe because we ensure that
    // whenever we hand out a cached node, we use a lifetime that cannot outlive
    // its owning `Cached<R>`.
    cached: RefCell<Option<Node<'static, Root>>>,
}

impl<R, Root> Cached<R, Root> {
    /// Construct a new `Cached<R>` of an inner `R`.
    ///
    /// # Example
    ///
    /// ```
    /// use bumpalo::Bump;
    /// use dodrio::{Cached, Node, Render};
    ///
    /// pub struct Counter {
    ///     count: u32,
    /// }
    ///
    /// impl Render for Counter {
    ///     fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    ///     where
    ///         'a: 'bump
    ///     {
    ///         // ...
    /// #       unimplemented!()
    ///     }
    /// }
    ///
    /// // Create a render-able counter.
    /// let counter = Counter { count: 0 };
    ///
    /// // And cache its rendering!
    /// let cached_counter = Cached::new(counter);
    /// ```
    #[inline]
    pub fn new(inner: R) -> Cached<R, Root> {
        let bump = Bump::new();
        let cached = RefCell::new(None);
        Cached {
            inner,
            bump,
            cached,
        }
    }

    /// Invalidate the cached rendering.
    ///
    /// This method should be called whenever the inner `R` must be re-rendered,
    /// and the cached `Node` from the last time `R::render` was invoked can no
    /// longer be re-used.
    ///
    /// # Example
    ///
    /// The `Cached<Hello>` component must have its cache invalidated whenever
    /// the `who` string is changed, or else the cached rendering whill keep
    /// displaying greetings to old `who`s.
    ///
    /// ```
    /// use bumpalo::Bump;
    /// use dodrio::{Cached, Node, Render};
    ///
    /// /// A component that renders to "<p>Hello, {who}!</p>"
    /// pub struct Hello {
    ///     who: String
    /// }
    ///
    /// impl Render for Hello {
    ///     fn render<'a, 'bump>(&'a self, bump: &'bump Bump) -> Node<'bump>
    ///     where
    ///         'a: 'bump,
    ///     {
    ///         Node::element(bump, "p", [], [
    ///             Node::text("Hello, "),
    ///             Node::text(&self.who),
    ///             Node::text("!"),
    ///         ])
    ///     }
    /// }
    ///
    /// /// Whenever a `Cached<Hello>`'s `who` is updated, we need to invalidate the
    /// /// cache so that we don't keep displaying greetings to old `who`s.
    /// pub fn set_who(hello: &mut Cached<Hello>, who: String) {
    ///     hello.who = who;
    ///     Cached::invalidate(hello);
    /// }
    /// ```
    #[inline]
    pub fn invalidate(cached: &mut Self) {
        *cached.cached.borrow_mut() = None;
    }

    /// Convert a `Cached<R>` back into a plain `R`.
    #[inline]
    pub fn into_inner(cached: Cached<R, Root>) -> R {
        cached.inner
    }
}

impl<R, Root> Deref for Cached<R, Root> {
    type Target = R;

    fn deref(&self) -> &R {
        &self.inner
    }
}

impl<R, Root> DerefMut for Cached<R, Root> {
    fn deref_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

unsafe fn extend_node_lifetime<'a, Root>(node: Node<'a, Root>) -> Node<'static, Root> {
    mem::transmute(node)
}

impl<R, Root> Render<Root> for Cached<R, Root>
where
    R: Render<Root>,
    Root: Render<Root>,
{
    fn render<'a, 'bump>(&'a self, _: &'bump Bump) -> Node<'bump, Root>
    where
        'a: 'bump,
    {
        let mut cached = self.cached.borrow_mut();

        if let Some(cached) = cached.as_ref() {
            // The cached node is actually a self-reference, so it has the `'a`
            // lifetime.
            let cached: Node<'a, Root> = cached.clone();

            // But the `'a` lifetime outlives `'bump`, so we can safely convert
            // it to the narrower `'bump` lifetime.
            let cached: Node<'bump, Root> = cached;

            // Return the cached rendering!
            return cached;
        }

        // We don't have a cached node. Render into our own `Bump`, cache it for
        // future renders, and return it. Same lifetimes as above.
        let node: Node<'a, Root> = self.inner.render(&self.bump);
        *cached = Some(unsafe { extend_node_lifetime(node.clone()) });
        let node: Node<'bump, Root> = node;
        node
    }
}
