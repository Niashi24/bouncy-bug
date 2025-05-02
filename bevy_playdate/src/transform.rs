use alloc::vec::Vec;
use bevy_app::{App, Plugin, PostStartup, PostUpdate};
use bevy_ecs::prelude::*;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use derive_more::{Add, AddAssign, Deref, DerefMut};

pub struct TransformPlugin;

/// Set enum for the systems relating to transform propagation
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum TransformSystem {
    /// Propagates changes in transform to children's [`GlobalTransform`](crate::components::GlobalTransform)
    TransformPropagate,
}

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Transform>()
            .register_type::<GlobalTransform>()
            .register_type::<TransformTreeChanged>();
        app
            // add transform systems to startup so the first update is "correct"
            .add_systems(
                PostStartup,
                (
                    mark_dirty_trees,
                    propagate_parent_transforms,
                    sync_simple_transforms,
                )
                    .chain()
                    .in_set(TransformSystem::TransformPropagate),
            )
            .add_systems(
                PostUpdate,
                (
                    mark_dirty_trees,
                    propagate_parent_transforms,
                    // TODO: Adjust the internal parallel queries to make this system more efficiently share and fill CPU time.
                    sync_simple_transforms,
                )
                    .chain()
                    .in_set(TransformSystem::TransformPropagate),
            );
    }
}

// note: much of this is copied from the `bevy_transform` source code and modified
// so that we only use a 2d position value

#[derive(
    Copy, Clone, PartialEq, Debug, Default, Deref, DerefMut, Component, Reflect, Add, AddAssign,
)]
#[reflect(Component)]
#[require(GlobalTransform, TransformTreeChanged)]
pub struct Transform(pub Vec2);

impl Transform {
    #[inline]
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self(Vec2::new(x, y))
    }
}

#[derive(
    Copy, Clone, PartialEq, Debug, Default, Deref, DerefMut, Component, Reflect, Add, AddAssign,
)]
#[reflect(Component)]
pub struct GlobalTransform(pub Vec2);

/// An optimization for transform propagation. This ZST marker component uses change detection to
/// mark all entities of the hierarchy as "dirty" if any of their descendants have a changed
/// `Transform`. If this component is *not* marked `is_changed()`, propagation will halt.
#[derive(Clone, Copy, Default, PartialEq, Debug, Component, Reflect)]
#[reflect(Component)]
pub struct TransformTreeChanged;

impl From<Transform> for GlobalTransform {
    fn from(value: Transform) -> Self {
        Self(value.0)
    }
}

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with
/// [`propagate_parent_transforms`] and [`mark_dirty_trees`].
#[allow(clippy::type_complexity)]
pub fn sync_simple_transforms(
    mut query: ParamSet<(
        Query<
            (&Transform, &mut GlobalTransform),
            (
                Or<(Changed<Transform>, Added<GlobalTransform>)>,
                Without<ChildOf>,
                Without<Children>,
            ),
        >,
        Query<(Ref<Transform>, &mut GlobalTransform), (Without<ChildOf>, Without<Children>)>,
    )>,
    mut orphaned: RemovedComponents<ChildOf>,
) {
    // Update changed entities.
    query
        .p0()
        .into_iter() // note: this was changed in
        .for_each(|(transform, mut global_transform)| {
            *global_transform = GlobalTransform::from(*transform);
        });
    // Update orphaned entities.
    let mut query = query.p1();
    let mut iter = query.iter_many_mut(orphaned.read());
    while let Some((transform, mut global_transform)) = iter.fetch_next() {
        if !transform.is_changed() && !global_transform.is_added() {
            *global_transform = GlobalTransform::from(*transform);
        }
    }
}

/// Optimization for static scenes. Propagates a "dirty bit" up the hierarchy towards ancestors.
/// Transform propagation can ignore entire subtrees of the hierarchy if it encounters an entity
/// without the dirty bit.
#[allow(clippy::type_complexity)]
pub fn mark_dirty_trees(
    changed_transforms: Query<
        Entity,
        Or<(Changed<Transform>, Changed<ChildOf>, Added<GlobalTransform>)>,
    >,
    mut orphaned: RemovedComponents<ChildOf>,
    mut transforms: Query<(Option<&ChildOf>, &mut TransformTreeChanged)>,
) {
    for entity in changed_transforms.iter().chain(orphaned.read()) {
        let mut next = entity;
        while let Ok((child_of, mut tree)) = transforms.get_mut(next) {
            if tree.is_changed() && !tree.is_added() {
                // If the component was changed, this part of the tree has already been processed.
                // Ignore this if the change was caused by the component being added.
                break;
            }
            tree.set_changed();
            if let Some(parent) = child_of.map(ChildOf::parent) {
                next = parent;
            } else {
                break;
            };
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn propagate_parent_transforms(
    mut root_query: Query<
        (Entity, &Children, Ref<Transform>, &mut GlobalTransform),
        Without<ChildOf>,
    >,
    mut orphaned: RemovedComponents<ChildOf>,
    transform_query: Query<
        (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
        With<ChildOf>,
    >,
    child_query: Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
    mut orphaned_entities: Local<Vec<Entity>>,
) {
    orphaned_entities.clear();
    orphaned_entities.extend(orphaned.read());
    orphaned_entities.sort_unstable();
    root_query.par_iter_mut().for_each(
        |(entity, children, transform, mut global_transform)| {
            let changed = transform.is_changed() || global_transform.is_added() || orphaned_entities.binary_search(&entity).is_ok();
            if changed {
                *global_transform = GlobalTransform::from(*transform);
            }

            for (child, child_of) in child_query.iter_many(children) {
                assert_eq!(
                    child_of.parent(), entity,
                    "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
                );
                // SAFETY:
                // - `child` must have consistent parentage, or the above assertion would panic.
                //   Since `child` is parented to a root entity, the entire hierarchy leading to it
                //   is consistent.
                // - We may operate as if all descendants are consistent, since
                //   `propagate_recursive` will panic before continuing to propagate if it
                //   encounters an entity with inconsistent parentage.
                // - Since each root entity is unique and the hierarchy is consistent and
                //   forest-like, other root entities' `propagate_recursive` calls will not conflict
                //   with this one.
                // - Since this is the only place where `transform_query` gets used, there will be
                //   no conflicting fetches elsewhere.
                #[expect(unsafe_code, reason = "`propagate_recursive()` is unsafe due to its use of `Query::get_unchecked()`.")]
                unsafe {
                    propagate_recursive(
                        &global_transform,
                        &transform_query,
                        &child_query,
                        child,
                        changed || child_of.is_changed(),
                    );
                }
            }
        },
    );
}

/// Recursively propagates the transforms for `entity` and all of its descendants.
///
/// # Panics
///
/// If `entity`'s descendants have a malformed hierarchy, this function will panic occur before
/// propagating the transforms of any malformed entities and their descendants.
///
/// # Safety
///
/// - While this function is running, `transform_query` must not have any fetches for `entity`,
///   nor any of its descendants.
/// - The caller must ensure that the hierarchy leading to `entity` is well-formed and must
///   remain as a tree or a forest. Each entity must have at most one parent.
#[expect(
    unsafe_code,
    reason = "This function uses `Query::get_unchecked()`, which can result in multiple mutable references if the preconditions are not met."
)]
#[allow(clippy::type_complexity)]
unsafe fn propagate_recursive(
    parent: &GlobalTransform,
    transform_query: &Query<
        (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
        With<ChildOf>,
    >,
    child_query: &Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
    entity: Entity,
    mut changed: bool,
) {
    let (global_matrix, children) = {
        let Ok((transform, mut global_transform, children)) =
            // SAFETY: This call cannot create aliased mutable references.
            //   - The top level iteration parallelizes on the roots of the hierarchy.
            //   - The caller ensures that each child has one and only one unique parent throughout
            //     the entire hierarchy.
            //
            // For example, consider the following malformed hierarchy:
            //
            //     A
            //   /   \
            //  B     C
            //   \   /
            //     D
            //
            // D has two parents, B and C. If the propagation passes through C, but the ChildOf
            // component on D points to B, the above check will panic as the origin parent does
            // match the recorded parent.
            //
            // Also consider the following case, where A and B are roots:
            //
            //  A       B
            //   \     /
            //    C   D
            //     \ /
            //      E
            //
            // Even if these A and B start two separate tasks running in parallel, one of them will
            // panic before attempting to mutably access E.
            (unsafe { transform_query.get_unchecked(entity) }) else {
                return;
            };

        changed |= transform.is_changed() || global_transform.is_added();
        if changed {
            *global_transform = *parent + GlobalTransform::from(*transform);
        }
        (global_transform, children)
    };

    let Some(children) = children else { return };
    for (child, child_of) in child_query.iter_many(children) {
        assert_eq!(
            child_of.parent(),
            entity,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        // SAFETY: The caller guarantees that `transform_query` will not be fetched for any
        // descendants of `entity`, so it is safe to call `propagate_recursive` for each child.
        //
        // The above assertion ensures that each child has one and only one unique parent
        // throughout the entire hierarchy.
        unsafe {
            propagate_recursive(
                global_matrix.as_ref(),
                transform_query,
                child_query,
                child,
                changed || child_of.is_changed(),
            );
        }
    }
}
