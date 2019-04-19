use crate::{
    cached_set::{CacheId, CachedSet},
    change_list::ChangeListBuilder,
    events::EventsRegistry,
    node::{Attribute, ElementNode, Listener, Node, NodeKind, TextNode},
};
use fxhash::{FxHashMap, FxHashSet};
use std::cmp::Ordering;
use std::ops::Range;
use std::u32;

// Diff the `old` node with the `new` node. Emits instructions to modify a
// physical DOM node that reflects `old` into something that reflects `new`.
//
// Upon entry to this function, the physical DOM node must be on the top of the
// change list stack:
//
//     [... node]
//
// The change list stack is in the same state when this function exits.
pub(crate) fn diff(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &Node,
    new: &Node,
    cached_roots: &mut FxHashSet<CacheId>,
) {
    match (&new.kind, &old.kind) {
        (
            &NodeKind::Text(TextNode { text: new_text }),
            &NodeKind::Text(TextNode { text: old_text }),
        ) => {
            if new_text != old_text {
                change_list.set_text(new_text);
            }
        }

        (&NodeKind::Text(_), &NodeKind::Element(_)) => {
            create(cached_set, change_list, registry, new, cached_roots);
            registry.remove_subtree(&old);
            change_list.replace_with();
        }

        (&NodeKind::Element(_), &NodeKind::Text(_)) => {
            create(cached_set, change_list, registry, new, cached_roots);
            // Note: text nodes cannot have event listeners, so we don't need to
            // remove the old node's listeners from our registry her.
            change_list.replace_with();
        }

        (
            &NodeKind::Element(ElementNode {
                key: _,
                tag_name: new_tag_name,
                listeners: new_listeners,
                attributes: new_attributes,
                children: new_children,
                namespace: new_namespace,
            }),
            &NodeKind::Element(ElementNode {
                key: _,
                tag_name: old_tag_name,
                listeners: old_listeners,
                attributes: old_attributes,
                children: old_children,
                namespace: old_namespace,
            }),
        ) => {
            if new_tag_name != old_tag_name || new_namespace != old_namespace {
                create(cached_set, change_list, registry, new, cached_roots);
                registry.remove_subtree(&old);
                change_list.replace_with();
                return;
            }
            diff_listeners(change_list, registry, old_listeners, new_listeners);
            diff_attributes(change_list, old_attributes, new_attributes);
            diff_children(
                cached_set,
                change_list,
                registry,
                old_children,
                new_children,
                cached_roots,
            );
        }

        // Both the new and old nodes are cached.
        (&NodeKind::Cached(ref new), &NodeKind::Cached(ref old)) => {
            cached_roots.insert(new.id);

            if new.id == old.id {
                // This is the same cached node, so nothing has changed!
                return;
            }

            let new = cached_set.get(new.id);
            let old = cached_set.get(old.id);
            diff(cached_set, change_list, registry, old, new, cached_roots);
        }

        // New cached node when the old node was not cached. In this scenario,
        // we assume that they are pretty different, and it isn't worth diffing
        // the subtrees, so we just create the new cached node afresh.
        (&NodeKind::Cached(ref c), _) => {
            cached_roots.insert(c.id);
            let new = cached_set.get(c.id);
            create(cached_set, change_list, registry, new, cached_roots);
            registry.remove_subtree(&old);
            change_list.replace_with();
        }

        // Old cached node and new non-cached node. Again, assume that they are
        // probably pretty different and create the new non-cached node afresh.
        (_, &NodeKind::Cached(_)) => {
            create(cached_set, change_list, registry, new, cached_roots);
            registry.remove_subtree(&old);
            change_list.replace_with();
        }
    }
}

// Diff event listeners between `old` and `new`.
//
// The listeners' node must be on top of the change list stack:
//
//     [... node]
//
// The change list stack is left unchanged.
fn diff_listeners(
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Listener],
    new: &[Listener],
) {
    'outer1: for new_l in new {
        unsafe {
            // Safety relies on removing `new_l` from the registry manually at
            // the end of its lifetime. This happens below in the `'outer2`
            // loop, and elsewhere in diffing when removing old dom trees.
            registry.add(new_l);
        }

        for old_l in old {
            if new_l.event == old_l.event {
                change_list.update_event_listener(new_l);
                continue 'outer1;
            }
        }

        change_list.new_event_listener(new_l);
    }

    'outer2: for old_l in old {
        registry.remove(old_l);

        for new_l in new {
            if new_l.event == old_l.event {
                continue 'outer2;
            }
        }
        change_list.remove_event_listener(old_l.event);
    }
}

// Diff a node's attributes.
//
// The attributes' node must be on top of the change list stack:
//
//     [... node]
//
// The change list stack is left unchanged.
fn diff_attributes(change_list: &mut ChangeListBuilder, old: &[Attribute], new: &[Attribute]) {
    // Do O(n^2) passes to add/update and remove attributes, since
    // there are almost always very few attributes.
    'outer: for new_attr in new {
        if new_attr.is_volatile() {
            change_list.set_attribute(new_attr.name, new_attr.value);
        } else {
            for old_attr in old {
                if old_attr.name == new_attr.name {
                    if old_attr.value != new_attr.value {
                        change_list.set_attribute(new_attr.name, new_attr.value);
                    }
                    continue 'outer;
                }
            }
            change_list.set_attribute(new_attr.name, new_attr.value);
        }
    }

    'outer2: for old_attr in old {
        for new_attr in new {
            if old_attr.name == new_attr.name {
                continue 'outer2;
            }
        }
        change_list.remove_attribute(old_attr.name);
    }
}

// Diff the given set of old and new children.
//
// The parent must be on top of the change list stack when this function is
// entered:
//
//     [... parent]
//
// the change list stack is in the same state when this function returns.
fn diff_children(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
) {
    if new.is_empty() {
        if !old.is_empty() {
            remove_all_children(change_list, registry, old);
        }
        return;
    }

    if old.is_empty() {
        create_and_append_children(cached_set, change_list, registry, new, cached_roots);
        return;
    }

    let new_is_keyed = new[0].key().is_some();
    let old_is_keyed = old[0].key().is_some();

    debug_assert!(
        new.iter().all(|n| n.key().is_some() == new_is_keyed),
        "all siblings must be keyed or all siblings must be non-keyed"
    );
    debug_assert!(
        old.iter().all(|o| o.key().is_some() == old_is_keyed),
        "all siblings must be keyed or all siblings must be non-keyed"
    );

    if new_is_keyed && old_is_keyed {
        let t = change_list.next_temporary();
        diff_keyed_children(cached_set, change_list, registry, old, new, cached_roots);
        change_list.set_next_temporary(t);
    } else {
        diff_non_keyed_children(cached_set, change_list, registry, old, new, cached_roots);
    }
}

// Diffing "keyed" children.
//
// With keyed children, we care about whether we delete, move, or create nodes
// versus mutate existing nodes in place. Presumably there is some sort of CSS
// transition animation that makes the virtual DOM diffing algorithm
// observable. By specifying keys for nodes, we know which virtual DOM nodes
// must reuse (or not reuse) the same physical DOM nodes.
//
// This is loosely based on Inferno's keyed patching implementation. However, we
// have to modify the algorithm since we are compiling the diff down into change
// list instructions that will be executed later, rather than applying the
// changes to the DOM directly as we compare virtual DOMs.
//
// https://github.com/infernojs/inferno/blob/36fd96/packages/inferno/src/DOM/patching.ts#L530-L739
//
// When entering this function, the parent must be on top of the change list
// stack:
//
//     [... parent]
//
// Upon exiting, the change list stack is in the same state.
fn diff_keyed_children(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
) {
    if cfg!(debug_assertions) {
        let mut keys = FxHashSet::default();
        let mut assert_unique_keys = |children: &[Node]| {
            keys.clear();
            for child in children {
                let key = child.key();
                debug_assert!(
                    key.is_some(),
                    "if any sibling is keyed, all siblings must be keyed"
                );
                keys.insert(key);
            }
            debug_assert_eq!(
                children.len(),
                keys.len(),
                "keyed siblings must each have a unique key"
            );
        };
        assert_unique_keys(old);
        assert_unique_keys(new);
    }

    // First up, we diff all the nodes with the same key at the beginning of the
    // children.
    //
    // `shared_prefix_count` is the count of how many nodes at the start of
    // `new` and `old` share the same keys.
    let shared_prefix_count =
        match diff_keyed_prefix(cached_set, change_list, registry, old, new, cached_roots) {
            KeyedPrefixResult::Finished => return,
            KeyedPrefixResult::MoreWorkToDo(count) => count,
        };

    // Next, we find out how many of the nodes at the end of the children have
    // the same key. We do _not_ diff them yet, since we want to emit the change
    // list instructions such that they can be applied in a single pass over the
    // DOM. Instead, we just save this information for later.
    //
    // `shared_suffix_count` is the count of how many nodes at the end of `new`
    // and `old` share the same keys.
    let shared_suffix_count = old[shared_prefix_count..]
        .iter()
        .rev()
        .zip(new[shared_prefix_count..].iter().rev())
        .take_while(|&(old, new)| old.key() == new.key())
        .count();

    let old_shared_suffix_start = old.len() - shared_suffix_count;
    let new_shared_suffix_start = new.len() - shared_suffix_count;

    // Ok, we now hopefully have a smaller range of children in the middle
    // within which to re-order nodes with the same keys, remove old nodes with
    // now-unused keys, and create new nodes with fresh keys.
    diff_keyed_middle(
        cached_set,
        change_list,
        registry,
        &old[shared_prefix_count..old_shared_suffix_start],
        &new[shared_prefix_count..new_shared_suffix_start],
        cached_roots,
        shared_prefix_count,
        shared_suffix_count,
        old_shared_suffix_start,
    );

    // Finally, diff the nodes at the end of `old` and `new` that share keys.
    let old_suffix = &old[old_shared_suffix_start..];
    let new_suffix = &new[new_shared_suffix_start..];
    debug_assert_eq!(old_suffix.len(), new_suffix.len());
    if !old_suffix.is_empty() {
        diff_keyed_suffix(
            cached_set,
            change_list,
            registry,
            old_suffix,
            new_suffix,
            cached_roots,
            new_shared_suffix_start,
        );
    }
}

enum KeyedPrefixResult {
    // Fast path: we finished diffing all the children just by looking at the
    // prefix of shared keys!
    Finished,
    // There is more diffing work to do. Here is a count of how many children at
    // the beginning of `new` and `old` we already processed.
    MoreWorkToDo(usize),
}

// Diff the prefix of children in `new` and `old` that share the same keys in
// the same order.
//
// Upon entry of this function, the change list stack must be:
//
//     [... parent]
//
// Upon exit, the change list stack is the same.
fn diff_keyed_prefix(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
) -> KeyedPrefixResult {
    let mut pushed = false;
    let mut shared_prefix_count = 0;

    for (old, new) in old.iter().zip(new.iter()) {
        if old.key() != new.key() {
            break;
        }

        if pushed {
            debug_assert!(shared_prefix_count > 0);
            change_list.pop_push_next_sibling();
        } else {
            debug_assert_eq!(shared_prefix_count, 0);
            change_list.push_first_child();
            pushed = true;
        }

        diff(cached_set, change_list, registry, old, new, cached_roots);
        shared_prefix_count += 1;

        // At the end of the loop, the change list stack looks like
        //
        //     [... parent child_we_just_diffed]
        debug_assert!(pushed);
    }

    // If that was all of the old children, then create and append the remaining
    // new children and we're finished.
    if shared_prefix_count == old.len() {
        debug_assert!(
            pushed,
            "we handle the case of empty children before calling this method, so
             `shared_prefix_count` must be greater than zero, and therefore we
               must have pushed in the above loop"
        );
        change_list.pop();
        create_and_append_children(
            cached_set,
            change_list,
            registry,
            &new[shared_prefix_count..],
            cached_roots,
        );
        return KeyedPrefixResult::Finished;
    }

    // And if that was all of the new children, then remove all of the remaining
    // old children and we're finished.
    if shared_prefix_count == new.len() {
        // Same as above.
        debug_assert!(pushed);
        change_list.pop_push_next_sibling();
        change_list.remove_self_and_next_siblings();
        return KeyedPrefixResult::Finished;
    }

    if pushed {
        debug_assert!(shared_prefix_count > 0);
        change_list.pop();
    }
    KeyedPrefixResult::MoreWorkToDo(shared_prefix_count)
}

// The most-general, expensive code path for keyed children diffing.
//
// We find the longest subsequence within `old` of children that are relatively
// ordered the same way in `new` (via finding a longest-increasing-subsequence
// of the old child's index within `new`). The children that are elements of
// this subsequence will remain in place, minimizing the number of DOM moves we
// will have to do.
//
// Upon entry to this function, the change list stack must be:
//
//     [... parent]
//
// Upon exit from this function, it will be restored to that same state.
fn diff_keyed_middle(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
    shared_prefix_count: usize,
    shared_suffix_count: usize,
    old_shared_suffix_start: usize,
) {
    // Should have already diffed the shared-key prefixes and suffixes.
    debug_assert_ne!(new.first().map(|n| n.key()), old.first().map(|o| o.key()));
    debug_assert_ne!(new.last().map(|n| n.key()), old.last().map(|o| o.key()));

    // The algorithm below relies upon using `u32::MAX` as a sentinel
    // value, so if we have that many new nodes, it won't work. This
    // check is a bit academic (hence only enabled in debug), since
    // wasm32 doesn't have enough address space to hold that many nodes
    // in memory.
    debug_assert!(new.len() < u32::MAX as usize);

    // Map from each `old` node's key to its index within `old`.
    let mut old_key_to_old_index = FxHashMap::default();
    old_key_to_old_index.reserve(old.len());
    old_key_to_old_index.extend(old.iter().enumerate().map(|(i, o)| (o.key(), i)));

    // The set of shared keys between `new` and `old`.
    let mut shared_keys = FxHashSet::default();
    // Map from each index in `new` to the index of the node in `old` that
    // has the same key.
    let mut new_index_to_old_index = Vec::with_capacity(new.len());
    new_index_to_old_index.extend(new.iter().map(|n| {
        let key = n.key();
        if let Some(&i) = old_key_to_old_index.get(&key) {
            shared_keys.insert(key);
            i
        } else {
            u32::MAX as usize
        }
    }));

    // If none of the old keys are reused by the new children, then we
    // remove all the remaining old children and create the new children
    // afresh.
    if shared_suffix_count == 0 && shared_keys.is_empty() {
        if shared_prefix_count == 0 {
            remove_all_children(change_list, registry, old);
        } else {
            change_list.pop_push_next_sibling();
            remove_self_and_next_siblings(change_list, registry, &old[shared_prefix_count..]);
        }
        create_and_append_children(cached_set, change_list, registry, new, cached_roots);
        return;
    }

    // The longest increasing subsequence within `new_index_to_old_index` in
    // reverse order (since `lis_with` adds the indices in reverse
    // order). We will leave these nodes in place in the DOM, and only move
    // nodes that are not part of the LIS. This results in the minimum
    // number of DOM nodes moved.
    let mut reverse_lis = Vec::with_capacity(new_index_to_old_index.len());
    let mut predecessors = vec![0; new_index_to_old_index.len()];
    let mut starts = vec![0; new_index_to_old_index.len()];
    longest_increasing_subsequence::lis_with(
        &new_index_to_old_index,
        &mut reverse_lis,
        |a, b| a < b,
        &mut predecessors,
        &mut starts,
    );

    // Save each of the old children whose keys are reused in the new
    // children.
    let mut old_index_to_temp = vec![u32::MAX; old.len()];
    let mut start = 0;
    loop {
        let end = (start..old.len())
            .find(|&i| {
                let key = old[i].key();
                !shared_keys.contains(&key)
            })
            .unwrap_or(old.len());

        if end - start > 0 {
            let mut t = change_list.save_children_to_temporaries(
                shared_prefix_count + start,
                shared_prefix_count + end,
            );
            for i in start..end {
                old_index_to_temp[i] = t;
                t += 1;
            }
        }

        debug_assert!(end <= old.len());
        if end == old.len() {
            break;
        } else {
            start = end + 1;
        }
    }

    // Remove any old children whose keys were not reused in the new
    // children. Remove from the end first so that we don't mess up indices.
    let mut removed_count = 0;
    for (i, old_child) in old.iter().enumerate().rev() {
        if !shared_keys.contains(&old_child.key()) {
            change_list.remove_child(i + shared_prefix_count);
            removed_count += 1;
        }
    }

    // Whether we have pushed a child onto the change list stack or not.
    let mut pushed = false;

    // Now iterate from the end of the new children back to the beginning,
    // moving old children to their new destination, diffing them, and
    // creating new children as necessary. Note that iterating in reverse
    // order lets us use `Node.prototype.insertBefore` to move/insert
    // children.
    if shared_suffix_count > 0 {
        change_list.push_child(old_shared_suffix_start - removed_count);
        pushed = true;
    }
    let mut segment_end = new.len();
    for lis_index in reverse_lis {
        // Because we use `u32::MAX` as a sentinel value representing "a child
        // with this key is not a member of `new`", we filter that out here.
        let old_index = new_index_to_old_index[lis_index];
        if old_index == u32::MAX as usize {
            continue;
        }

        // A segment begins with a member of the LIS, which will remain in
        // place in the DOM, followed by zero or more new children that are
        // not members of the LIS. If one of these new children shares a key
        // with one of the old children, then we will move that old child in
        // the DOM. Otherwise, we create the new child afresh.
        debug_assert!(segment_end - lis_index > 0);

        // Go through each of the non-LIS child and move-and-diff or create
        // it.
        diff_and_move_or_create_segment(
            cached_set,
            change_list,
            registry,
            old,
            new,
            cached_roots,
            lis_index + 1..segment_end,
            &new_index_to_old_index,
            &old_index_to_temp,
            &mut pushed,
        );

        // Now we push the member of the LIS and diff it with its
        // correspondingly-keyed new child.
        let new_child = &new[lis_index];
        let temp = old_index_to_temp[old_index];
        debug_assert_ne!(temp, u32::MAX);
        if pushed {
            change_list.pop();
        }
        change_list.push_temporary(temp);
        pushed = true;
        diff(
            cached_set,
            change_list,
            registry,
            &old[old_index],
            new_child,
            cached_roots,
        );

        segment_end = lis_index;
    }

    // And now diff-and-move or create each of the non-LIS children that
    // appear before the first LIS-member.
    diff_and_move_or_create_segment(
        cached_set,
        change_list,
        registry,
        old,
        new,
        cached_roots,
        0..segment_end,
        &new_index_to_old_index,
        &old_index_to_temp,
        &mut pushed,
    );

    if pushed {
        change_list.pop();
    }
}

// Diff-and-then-move or create each new node in the given segment. The
// segment's nodes must _not_ be members of the LIS (those nodes we will diff in
// place elsewhere, and we should not move them nor create them afresh).
//
// On entering this function, the change list stack has the parent and
// optionally the next sibling after this segment of children:
//
//     [... parent next_sibling]     if *pushed == true
//     [... parent]                  otherwise
//
// After exiting, if any children were moved or created, then `pushed` is set to
// true and the change list stack has the first child in this segment on
// top. Otherwise the stack is left as it was on entering.
//
//     [... parent child]     if *pushed == true
//     [... parent]           otherwise
fn diff_and_move_or_create_segment(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
    segment: Range<usize>,
    new_index_to_old_index: &[usize],
    old_index_to_temp: &[u32],
    pushed: &mut bool,
) {
    for new_index in segment.rev() {
        let new_child = &new[new_index];
        let old_index = new_index_to_old_index[new_index];
        if old_index == u32::MAX as usize {
            // The key is not reused. Create this new child afresh.
            create(cached_set, change_list, registry, new_child, cached_roots);
        } else {
            // The key is reused. Diff it with the old node and then
            // move the old node into its final destination.
            let temp = old_index_to_temp[old_index];
            debug_assert_ne!(temp, u32::MAX);
            change_list.push_temporary(temp);
            diff(
                cached_set,
                change_list,
                registry,
                new_child,
                &old[old_index],
                cached_roots,
            );
        }

        // At this point the change list stack can have one of two shapes.
        if *pushed {
            // [... parent next_child new_child]
            change_list.insert_before();
        // [... parent new_child]
        } else {
            // [... parent new_child]
            change_list.append_child();
            // [... parent]
            change_list.push_last_child();
            // [... parent new_child]
            *pushed = true;
        }

        // At the end of each loop iteration, the change list stack looks like:
        //
        //     [... parent new_child]
        debug_assert!(*pushed);
    }
}

// Diff the suffix of keyed children that share the same keys in the same order.
//
// The parent must be on the change list stack when we enter this function:
//
//     [... parent]
//
// When this function exits, the change list stack remains the same.
fn diff_keyed_suffix(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
    new_shared_suffix_start: usize,
) {
    debug_assert_eq!(old.len(), new.len());
    debug_assert!(!old.is_empty());

    // [... parent]
    change_list.push_child(new_shared_suffix_start);
    // [... parent new_child]

    for (old_child, new_child) in old.iter().zip(new.iter()) {
        diff(
            cached_set,
            change_list,
            registry,
            old_child,
            new_child,
            cached_roots,
        );

        // [... parent this_new_child]
        change_list.pop_push_next_sibling();
        // [... parent next_new_child]
    }

    // [... parent]
    change_list.pop();
}

// Diff children that are not keyed.
//
// The parent must be on the top of the change list stack when entering this
// function:
//
//     [... parent]
//
// the change list stack is in the same state when this function returns.
fn diff_non_keyed_children(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
) {
    // Handled these cases in `diff_children` before calling this function.
    debug_assert!(!new.is_empty());
    debug_assert!(!old.is_empty());

    for (i, (new_child, old_child)) in new.iter().zip(old.iter()).enumerate() {
        if i == 0 {
            // [... parent]
            change_list.push_first_child();
        // [... parent first_child]
        } else {
            // [... parent prev_sibling]
            change_list.pop_push_next_sibling();
            // [... parent next_sibling]
        }

        diff(
            cached_set,
            change_list,
            registry,
            old_child,
            new_child,
            cached_roots,
        );
    }

    // Note that because `new` and `old` are not empty, the previous loop always
    // executes at least once, so the change list stack is now:
    //
    //     [... parent child]

    match old.len().cmp(&new.len()) {
        Ordering::Greater => {
            // [... parent last_shared_child]
            change_list.pop_push_next_sibling();
            // [... parent first_child_to_remove]
            remove_self_and_next_siblings(change_list, registry, &old[new.len()..]);
            // [... parent]
        }
        Ordering::Less => {
            // [... parent last_child]
            change_list.pop();
            // [... parent]
            create_and_append_children(
                cached_set,
                change_list,
                registry,
                &new[old.len()..],
                cached_roots,
            );
        }
        Ordering::Equal => {
            // [... parent child]
            change_list.pop();
            // [... parent]
        }
    }
}

// Create the given children and append them to the parent node.
//
// The parent node must currently be on top of the change list stack:
//
//     [... parent]
//
// When this function returns, the change list stack is in the same state.
fn create_and_append_children(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    new: &[Node],
    cached_roots: &mut FxHashSet<CacheId>,
) {
    for child in new {
        create(cached_set, change_list, registry, child, cached_roots);
        change_list.append_child();
    }
}

// Remove all of a node's children.
//
// The change list stack must have this shape upon entry to this function:
//
//     [... parent]
//
// When this function returns, the change list stack is in the same state.
fn remove_all_children(
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
) {
    for child in old {
        registry.remove_subtree(child);
    }
    // Fast way to remove all children: set the node's textContent to an empty
    // string.
    change_list.set_text("");
}

// Remove the current child and all of its following siblings.
//
// The change list stack must have this shape upon entry to this function:
//
//     [... parent child]
//
// After the function returns, the child is no longer on the change list stack:
//
//     [... parent]
fn remove_self_and_next_siblings(
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    old: &[Node],
) {
    for child in old {
        registry.remove_subtree(child);
    }
    change_list.remove_self_and_next_siblings();
}

// Emit instructions to create the given virtual node.
//
// The change list stack may have any shape upon entering this function:
//
//     [...]
//
// When this function returns, the new node is on top of the change list stack:
//
//     [... node]
fn create(
    cached_set: &CachedSet,
    change_list: &mut ChangeListBuilder,
    registry: &mut EventsRegistry,
    node: &Node,
    cached_roots: &mut FxHashSet<CacheId>,
) {
    match node.kind {
        NodeKind::Text(TextNode { text }) => {
            change_list.create_text_node(text);
        }
        NodeKind::Element(&ElementNode {
            key: _,
            tag_name,
            listeners,
            attributes,
            children,
            namespace,
        }) => {
            if let Some(namespace) = namespace {
                change_list.create_element_ns(tag_name, namespace);
            } else {
                change_list.create_element(tag_name);
            }
            for l in listeners {
                unsafe {
                    registry.add(l);
                }
                change_list.new_event_listener(l);
            }
            for attr in attributes {
                change_list.set_attribute(&attr.name, &attr.value);
            }
            for child in children {
                create(cached_set, change_list, registry, child, cached_roots);
                change_list.append_child();
            }
        }
        NodeKind::Cached(ref c) => {
            cached_roots.insert(c.id);
            let node = cached_set.get(c.id);
            create(cached_set, change_list, registry, node, cached_roots)
        }
    }
}
