use crate::{
    cached_set::CachedSet,
    node::{CachedNode, NodeKey},
    Node, Render, RenderContext,
};
use std::any::TypeId;
use std::cell::Cell;
use std::ops::{Deref, DerefMut};

/// A renderable that supports caching for when rendering is expensive but can
/// generate the same DOM tree.
///
/// # Example
///
/// The [TodoMVC
/// example](https://github.com/fitzgen/dodrio/blob/master/examples/todomvc/src/todo.rs)
/// caches individual todo items.
#[derive(Clone, Debug, Default)]
pub struct Cached<R>
where
    R: Default,
{
    inner: R,
    cached: Cell<Option<CachedNode>>,
}

pub_unstable_internal! {
    #[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
    pub(crate) struct TemplateId(TypeId);
}

impl<R> Cached<R>
where
    R: Default,
{
    /// Construct a new `Cached<R>` of an inner `R`.
    ///
    /// # Example
    ///
    /// ```
    /// use dodrio::{Cached, Node, Render, RenderContext};
    ///
    /// #[derive(Default)]
    /// pub struct Counter {
    ///     count: u32,
    /// }
    ///
    /// impl<'a> Render<'a> for Counter {
    ///     fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
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
    pub fn new(inner: R) -> Cached<R> {
        let cached = Cell::new(None);
        Cached { inner, cached }
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
    /// the `who` string is changed, or else the cached rendering will keep
    /// displaying greetings to old `who`s.
    ///
    /// ```
    /// use dodrio::{bumpalo, Cached, Node, Render, RenderContext};
    ///
    /// /// A component that renders to "<p>Hello, {who}!</p>"
    /// #[derive(Default)]
    /// pub struct Hello {
    ///     who: String
    /// }
    ///
    /// impl<'a> Render<'a> for Hello {
    ///     fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
    ///         use dodrio::builder::*;
    ///         let greeting = bumpalo::format!(in cx.bump, "Hello, {}!", self.who);
    ///         p(&cx)
    ///             .children([text(greeting.into_bump_str())])
    ///             .finish()
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
    pub fn invalidate(cached: &Self) {
        cached.cached.set(None);
    }

    /// Convert a `Cached<R>` back into a plain `R`.
    #[inline]
    pub fn into_inner(cached: Self) -> R {
        cached.inner
    }
}

impl<R> Cached<R>
where
    R: 'static + Default,
{
    pub(crate) fn template_id() -> TemplateId {
        TemplateId(TypeId::of::<R>())
    }
}

impl<R> Deref for Cached<R>
where
    R: Default,
{
    type Target = R;

    fn deref(&self) -> &R {
        &self.inner
    }
}

impl<R> DerefMut for Cached<R>
where
    R: Default,
{
    fn deref_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl<'a, R> Render<'a> for Cached<R>
where
    R: 'static + Default + for<'b> Render<'b>,
{
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        let template = cx.template::<R>();
        let cached = match self.cached.get() {
            // This does-the-cache-contain-this-id check is necessary because
            // the same `Cached<R>` instance can be rendered into vdom A, which
            // will save the results into A's cached set and yield id X. Then,
            // it can be rendered *again* into a second vdom B, and B's cached
            // set does not have the saved results for id X. If we didn't have
            // this check, instead of a cache miss, we would have a panic.
            //
            // This scenario should basically never happen in the real world,
            // however. If we ever find that it is worth sharing a cached render
            // component between multiple vdoms, and want to avoid these
            // "unnecessary" cache misses, we can do this:
            //
            // * Make each `Cached<R>` have an instance id and generation
            //   counter.
            //
            // * On invalidation, bump the generation counter.
            //
            // * Add a generation member to the `CacheEntry`.
            //
            // * Each vdom maintains a map from `Cached<R>` instance id to
            // `CacheEntry`s.
            //
            // * We only re-use the cached results if the `Cached<R>`'s
            //   generation counter matches the entry in the vdom's cached set.
            //
            // This is all do-able but is a bit more book keeping than we really
            // want to do unless it is well motivated.
            Some(cached)
                if {
                    let cached_set = cx.cached_set.borrow();
                    cached_set.contains(cached.id)
                } =>
            {
                cached
            }
            _ => {
                let mut key = NodeKey::NONE;
                let id = CachedSet::insert(cx, false, template, |nested_cx| {
                    let node = self.inner.render(nested_cx);
                    key = node.key();
                    node
                });
                let cached = CachedNode { id, key };
                self.cached.set(Some(cached));
                cached
            }
        };

        cached.into()
    }
}

#[cfg(feature = "serde")]
impl<R> serde::Serialize for Cached<R>
where
    R: Default + serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, R> serde::Deserialize<'de> for Cached<R>
where
    R: Default + serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        R::deserialize(deserializer).map(Cached::new)
    }
}
