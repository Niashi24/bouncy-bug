use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::hierarchy::validate_parent_has_component;
use bevy_reflect::prelude::ReflectDefault;
use bevy_ecs::prelude::{Changed, ChildOf, Children, Component, Entity, Or, Query, With};
use bevy_ecs::query::Added;
use bevy_reflect::Reflect;
use derive_more::Deref;
use crate::sprite::{Sprite, SpriteSystemSet};

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                visibility_propagate_system,
                sync_sprite_visibility,
            ).chain()
                .before(SpriteSystemSet)
        );
    }
}

/// User indication of whether an entity is visible. Propagates down the entity hierarchy.
///
/// If an entity is hidden in this way, all [`Children`] (and all of their children and so on) who
/// are set to [`Inherited`](Self::Inherited) will also be hidden.
///
/// This is done by the `visibility_propagate_system` which uses the entity hierarchy and
/// `Visibility` to set the values of each entity's [`InheritedVisibility`] component.
#[derive(Component, Clone, Copy, Reflect, Debug, PartialEq, Eq, Default)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(InheritedVisibility)]
pub enum Visibility {
    /// An entity with `Visibility::Inherited` will inherit the Visibility of its [`ChildOf`] target.
    ///
    /// A root-level entity that is set to `Inherited` will be visible.
    #[default]
    Inherited,
    /// An entity with `Visibility::Hidden` will be unconditionally hidden.
    Hidden,
    /// An entity with `Visibility::Visible` will be unconditionally visible.
    ///
    /// Note that an entity with `Visibility::Visible` will be visible regardless of whether the
    /// [`ChildOf`] target entity is hidden.
    Visible,
}

impl Visibility {
    pub fn inherited_or_hidden(inherited: bool) -> Self {
        match inherited {
            false => Self::Hidden,
            true => Self::Inherited,
        }
    }
}

/// Whether an entity is visible in the hierarchy.
/// This will not be accurate until [`visibility_propagate_system`]
/// runs in the [`PostUpdate`] schedule.
#[derive(Component, Deref, Debug, Default, Clone, Copy, Reflect, PartialEq, Eq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[component(on_insert = validate_parent_has_component::<Self>)]
pub struct InheritedVisibility(bool);

impl InheritedVisibility {
    /// An entity that is invisible in the hierarchy.
    pub const HIDDEN: Self = Self(false);
    /// An entity that is visible in the hierarchy.
    pub const VISIBLE: Self = Self(true);
    
    /// Returns `true` if the entity is visible in the hierarchy.
    /// Otherwise, returns `false`.
    #[inline]
    pub fn get(self) -> bool {
        self.0
    }
}

pub fn visibility_propagate_system(
    changed: Query<
        (Entity, &Visibility, Option<&ChildOf>, Option<&Children>),
        (
            With<InheritedVisibility>,
            Or<(Changed<Visibility>, Changed<ChildOf>)>,
        ),
    >,
    mut visibility_query: Query<(&Visibility, &mut InheritedVisibility)>,
    children_query: Query<&Children, (With<Visibility>, With<InheritedVisibility>)>,
) {
    for (entity, visibility, child_of, children) in &changed {
        let is_visible = match visibility {
            Visibility::Visible => true,
            Visibility::Hidden => false,
            // fall back to true if no parent is found or parent lacks components
            Visibility::Inherited => child_of
                .and_then(|c| visibility_query.get(c.parent()).ok())
                .is_none_or(|(_, x)| x.get()),
        };
        let (_, mut inherited_visibility) = visibility_query
            .get_mut(entity)
            .expect("With<InheritedVisibility> ensures this query will return a value");
        
        // Only update the visibility if it has changed.
        // This will also prevent the visibility from propagating multiple times in the same frame
        // if this entity's visibility has been updated recursively by its parent.
        if inherited_visibility.get() != is_visible {
            inherited_visibility.0 = is_visible;
            
            // Recursively update the visibility of each child.
            for &child in children.into_iter().flatten() {
                let _ =
                    propagate_recursive(is_visible, child, &mut visibility_query, &children_query);
            }
        }
    }
}

fn propagate_recursive(
    parent_is_visible: bool,
    entity: Entity,
    visibility_query: &mut Query<(&Visibility, &mut InheritedVisibility)>,
    children_query: &Query<&Children, (With<Visibility>, With<InheritedVisibility>)>,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    // Get the visibility components for the current entity.
    // If the entity does not have the required components, just return early.
    let (visibility, mut inherited_visibility) = visibility_query.get_mut(entity).map_err(drop)?;
    
    let is_visible = match visibility {
        Visibility::Visible => true,
        Visibility::Hidden => false,
        Visibility::Inherited => parent_is_visible,
    };
    
    // Only update the visibility if it has changed.
    if inherited_visibility.get() != is_visible {
        inherited_visibility.0 = is_visible;
        
        // Recursively update the visibility of each child.
        for &child in children_query.get(entity).ok().into_iter().flatten() {
            let _ = propagate_recursive(is_visible, child, visibility_query, children_query);
        }
    }
    
    Ok(())
}

pub fn sync_sprite_visibility(
    mut q_sprite: Query<(&mut Sprite, &InheritedVisibility), Or<(Changed<InheritedVisibility>, Added<Sprite>)>>,
) {
    for (sprite, visibility) in q_sprite.iter_mut() {
        sprite.set_visible(visibility.get());
    }
}

