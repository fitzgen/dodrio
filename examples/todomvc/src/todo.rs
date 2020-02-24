//! Type definition and `dodrio::Render` implementation for a single todo item.

use crate::keys;
use dodrio::{Cached, Node, Render, RenderContext, RootRender, VdomWeak};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use wasm_bindgen::{prelude::*, JsCast};

/// A single todo item.
#[derive(Default)]
pub struct Todo<C> {
    inner: Cached<TodoInner<C>>,
}

#[derive(Serialize, Deserialize)]
struct TodoInner<C> {
    id: usize,
    title: String,
    completed: bool,

    #[serde(skip)]
    edits: Option<String>,

    #[serde(skip)]
    _controller: PhantomData<C>,
}

/// Actions on a single todo item that can be triggered from the UI.
pub trait TodoActions {
    /// Toggle the completion state of the todo item with the given id.
    fn toggle_completed(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Delete the todo item with the given id.
    fn delete(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Begin editing the todo item with the given id.
    fn begin_editing(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Update the edits for the todo with the given id.
    fn update_edits(root: &mut dyn RootRender, vdom: VdomWeak, id: usize, edits: String);

    /// Finish editing the todo with the given id.
    fn finish_edits(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);

    /// Cancel editing the todo with the given id.
    fn cancel_edits(root: &mut dyn RootRender, vdom: VdomWeak, id: usize);
}

impl<C> Todo<C> {
    /// Construct a new `Todo` with the given identifier and title.
    pub fn new<S: Into<String>>(id: usize, title: S) -> Self {
        let title = title.into();
        let completed = false;
        let edits = None;
        Todo {
            inner: Cached::new(TodoInner {
                id,
                title,
                completed,
                edits,
                _controller: PhantomData,
            }),
        }
    }

    /// Set this todo item's id.
    pub fn set_id(&mut self, id: usize) {
        if id != self.inner.id {
            Cached::invalidate(&self.inner);
        }
        self.inner.id = id;
    }

    /// Is this `Todo` complete?
    pub fn is_complete(&self) -> bool {
        self.inner.completed
    }

    /// Mark the `Todo` as complete or not.
    pub fn set_complete(&mut self, to: bool) {
        if to != self.inner.completed {
            Cached::invalidate(&self.inner);
        }
        self.inner.completed = to;
    }

    /// Get this todo's title.
    pub fn title(&self) -> &str {
        &self.inner.title
    }

    /// Set this todo item's title.
    pub fn set_title<S: Into<String>>(&mut self, title: S) {
        let title = title.into();
        if title != self.inner.title {
            Cached::invalidate(&self.inner);
        }
        self.inner.title = title;
    }

    /// Set the edits for this todo.
    pub fn set_edits<S: Into<String>>(&mut self, edits: Option<S>) {
        let edits = edits.map(Into::into);
        if edits != self.inner.edits {
            Cached::invalidate(&self.inner);
        }
        self.inner.edits = edits;
    }

    /// Take this todo's edits, leaving `None` in their place.
    pub fn take_edits(&mut self) -> Option<String> {
        if self.inner.edits.is_some() {
            Cached::invalidate(&self.inner);
        }
        self.inner.edits.take()
    }
}

impl<'a, C: TodoActions> Render<'a> for Todo<C> {
    fn render(&self, cx: &mut RenderContext<'a>) -> Node<'a> {
        use dodrio::{
            builder::*,
            bumpalo::{self, collections::String},
        };

        let id = self.inner.id;
        let title = self.inner.edits.as_ref().unwrap_or(&self.inner.title);
        let title = bumpalo::format!(in cx.bump, "{}", title).into_bump_str();

        li(&cx)
            .attr("class", {
                let mut class = String::new_in(cx.bump);
                if self.inner.completed {
                    class.push_str("completed ");
                }
                if self.inner.edits.is_some() {
                    class.push_str("editing");
                }
                class.into_bump_str()
            })
            .children([
                div(&cx)
                    .attr("class", "view")
                    .children([
                        input(&cx)
                            .attr("class", "toggle")
                            .attr("type", "checkbox")
                            .bool_attr("checked", self.inner.completed)
                            .on("click", move |root, vdom, _event| {
                                C::toggle_completed(root, vdom, id);
                            })
                            .finish(),
                        label(&cx)
                            .on("dblclick", move |root, vdom, _event| {
                                C::begin_editing(root, vdom, id);
                            })
                            .children([text(title)])
                            .finish(),
                        button(&cx)
                            .attr("class", "destroy")
                            .on("click", move |root, vdom, _event| {
                                C::delete(root, vdom, id);
                            })
                            .finish(),
                    ])
                    .finish(),
                input(&cx)
                    .attr("class", "edit")
                    .attr("value", title)
                    .attr("name", "title")
                    .attr(
                        "id",
                        bumpalo::format!(in cx.bump, "todo-{}", id).into_bump_str(),
                    )
                    .on("input", move |root, vdom, event| {
                        let input = event
                            .target()
                            .unwrap_throw()
                            .unchecked_into::<web_sys::HtmlInputElement>();
                        C::update_edits(root, vdom, id, input.value());
                    })
                    .on("blur", move |root, vdom, _event| {
                        C::finish_edits(root, vdom, id);
                    })
                    .on("keydown", move |root, vdom, event| {
                        let event = event.unchecked_into::<web_sys::KeyboardEvent>();
                        match event.key_code() {
                            keys::ENTER => C::finish_edits(root, vdom, id),
                            keys::ESCAPE => C::cancel_edits(root, vdom, id),
                            _ => {}
                        }
                    })
                    .finish(),
            ])
            .finish()
    }
}

impl<C> Default for TodoInner<C> {
    fn default() -> Self {
        TodoInner {
            id: Default::default(),
            title: Default::default(),
            completed: Default::default(),
            edits: Default::default(),
            _controller: PhantomData,
        }
    }
}

impl<C> serde::Serialize for Todo<C> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, C> Deserialize<'de> for Todo<C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Todo {
            inner: <Cached<TodoInner<C>>>::deserialize(deserializer)?,
        })
    }
}
