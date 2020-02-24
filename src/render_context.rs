use crate::{
    cached::{Cached, TemplateId},
    cached_set::{CacheId, CachedSet},
    Node, Render,
};
use bumpalo::Bump;
use fxhash::FxHashMap;
use std::fmt;

/// Common context available to all `Render` implementations.
///
/// Notably, the `RenderContext` gives access to the bump arena that the virtual
/// DOM should be allocated within. This is available via the `bump` field.
pub struct RenderContext<'a> {
    /// The underlying bump arena that virtual DOMs are rendered into.
    ///
    /// ## Example
    ///
    /// ```
    /// use dodrio::RenderContext;
    ///
    /// // Given a rendering context, allocate an i32 inside its bump arena.
    /// fn foo<'a>(cx: &mut RenderContext<'a>) -> &'a mut i32 {
    ///     cx.bump.alloc(42)
    /// }
    /// ```
    pub bump: &'a Bump,

    pub(crate) cached_set: &'a crate::RefCell<CachedSet>,

    pub(crate) templates: &'a mut FxHashMap<TemplateId, Option<CacheId>>,

    // Prevent exhaustive matching on the rendering context, so we can always
    // add more members in a semver-compatible way.
    _non_exhaustive: (),
}

impl fmt::Debug for RenderContext<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RenderContext")
            .field("bump", &self.bump)
            .finish()
    }
}

impl<'a> RenderContext<'a> {
    pub_unstable_internal! {
        pub(crate) fn new(
            bump: &'a Bump,
            cached_set: &'a crate::RefCell<CachedSet>,
            templates: &'a mut FxHashMap<TemplateId, Option<CacheId>>
        ) -> Self {
            RenderContext {
                bump,
                cached_set,
                templates,
                _non_exhaustive: (),
            }
        }
    }

    pub(crate) fn cache<F>(&mut self, pinned: bool, template: Option<CacheId>, f: F) -> CacheId
    where
        F: for<'b> FnOnce(&mut RenderContext<'b>) -> Node<'b>,
    {
        CachedSet::insert(self, pinned, template, f)
    }

    /// Get or create the cached template for `Cached<R>`.
    pub(crate) fn template<R>(&mut self) -> Option<CacheId>
    where
        R: 'static + Default + for<'b> Render<'b>,
    {
        let template_id = Cached::<R>::template_id();
        if let Some(cache_id) = self.templates.get(&template_id).cloned() {
            return cache_id;
        }

        // Prevent re-entrancy from infinite looping. Any attempts to get `R`'s
        // template while constructing the template will simply fail to use the
        // templated fast path.
        self.templates.insert(template_id, None);

        // Render the default `R` and save that as the template for all
        // `Cached<R>`s.
        let cache_id = self.cache(true, None, |nested_cx| R::default().render(nested_cx));
        self.templates.insert(template_id, Some(cache_id));
        Some(cache_id)
    }
}

impl<'a, 'b> From<&'b RenderContext<'a>> for &'a Bump {
    #[inline]
    fn from(cx: &'b RenderContext<'a>) -> &'a Bump {
        cx.bump
    }
}

impl<'a, 'b, 'c> From<&'c &'b mut RenderContext<'a>> for &'a Bump {
    #[inline]
    fn from(cx: &'c &'b mut RenderContext<'a>) -> &'a Bump {
        cx.bump
    }
}
