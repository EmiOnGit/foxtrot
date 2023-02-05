use bevy::prelude::*;

use bevy_rapier3d::prelude::*;
mod components;
use crate::level_instanciation::spawning::AnimationEntityLink;
use crate::util::trait_extension::Vec3Ext;
use crate::GameState;
pub use components::{Velocity, *};

pub struct GeneralMovementPlugin;

impl Plugin for GeneralMovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Model>()
            .register_type::<Grounded>()
            .register_type::<Jump>()
            .register_type::<Velocity>()
            .register_type::<Drag>()
            .register_type::<Walker>()
            .add_system_set(
                SystemSet::on_update(GameState::Playing)
                    .with_system(update_grounded.label("update_grounded"))
                    .with_system(
                        apply_gravity
                            .label("apply_gravity")
                            .after("update_grounded")
                            .before("apply_force"),
                    )
                    .with_system(
                        apply_walking
                            .label("apply_walking")
                            .after("update_grounded")
                            .before("apply_force"),
                    )
                    .with_system(
                        apply_jumping
                            .label("apply_jumping")
                            .after("update_grounded")
                            .before("apply_force"),
                    )
                    .with_system(
                        apply_drag
                            .label("apply_drag")
                            .after("apply_walking")
                            .after("apply_gravity")
                            .before("apply_force"),
                    )
                    .with_system(apply_force.label("apply_force"))
                    .with_system(
                        reset_movement_components
                            .label("reset_movement_components")
                            .after("apply_force"),
                    )
                    .with_system(rotate_model)
                    .with_system(play_animations),
            );
    }
}

fn update_grounded(mut query: Query<(&mut Grounded, &KinematicCharacterControllerOutput)>) {
    for (mut grounded, output) in &mut query {
        grounded.try_set(output.grounded);
    }
}

fn apply_gravity(mut character: Query<(&mut Force, &KinematicCharacterController, &Mass, &Jump)>) {
    for (mut force, controller, mass, jump) in &mut character {
        let gravitational_force = -controller.up * jump.g * mass.0;
        force.0 += gravitational_force;
    }
}

/// Treat `Force` as readonly after this system.
fn apply_force(
    time: Res<Time>,
    mut player_query: Query<(
        &Force,
        &mut Velocity,
        &mut KinematicCharacterController,
        &Mass,
    )>,
) {
    let dt = time.delta_seconds();
    for (force, mut velocity, mut controller, mass) in &mut player_query {
        let acceleration = force.0 / mass.0;
        let desired_translation = velocity.0 * dt + 0.5 * acceleration * dt * dt;
        velocity.0 += acceleration * dt;
        controller.translation = Some(desired_translation);
    }
}

fn reset_movement_components(
    mut forces: Query<&mut Force>,
    mut walkers: Query<&mut Walker>,
    mut jumpers: Query<&mut Jump>,
) {
    for mut force in &mut forces {
        force.0 = Vec3::ZERO;
    }
    for mut walker in &mut walkers {
        walker.direction = None;
    }
    for mut jumper in &mut jumpers {
        jumper.requested = false;
    }
}

fn apply_jumping(
    time: Res<Time>,
    mut character_query: Query<(&Grounded, &mut Force, &KinematicCharacterController, &Jump)>,
) {
    let dt = time.delta_seconds();
    for (grounded, mut force, controller, jump) in &mut character_query {
        if jump.requested && grounded.is_grounded() {
            force.0 += controller.up * jump.impulse / dt;
        }
    }
}

fn rotate_model(
    player_query: Query<(&KinematicCharacterControllerOutput, &AnimationEntityLink)>,
    mut transforms: Query<&mut Transform>,
) {
    for (output, link) in player_query.iter() {
        let horizontal_movement = output.effective_translation.x0z();
        if horizontal_movement.is_approx_zero() {
            continue;
        }
        let mut transform = transforms.get_mut(link.0).unwrap();
        *transform = transform.looking_at(transform.translation + horizontal_movement, Vec3::Y);
    }
}

fn play_animations(
    mut animation_player: Query<&mut AnimationPlayer>,
    characters: Query<(
        &KinematicCharacterControllerOutput,
        &Grounded,
        &AnimationEntityLink,
        &CharacterAnimations,
    )>,
) {
    for (output, grounded, animation_entity_link, animations) in characters.iter() {
        let mut animation_player = animation_player
            .get_mut(animation_entity_link.0)
            .expect("animation_entity_link held entity without animation player");

        let has_horizontal_movement = !output.effective_translation.x0z().is_approx_zero();

        if !grounded.is_grounded() {
            animation_player
                .play(animations.aerial.clone_weak())
                .repeat();
        } else if has_horizontal_movement {
            animation_player.play(animations.walk.clone_weak()).repeat();
        } else {
            animation_player.play(animations.idle.clone_weak()).repeat();
        }
    }
}

fn apply_drag(mut character_query: Query<(&mut Force, &Velocity, &Drag)>) {
    for (mut force, velocity, drag) in &mut character_query {
        let drag_force = drag.calculate_force(velocity.0);
        force.0 += drag_force;
    }
}

fn apply_walking(mut character_query: Query<(&mut Force, &Walker, &Grounded, &Mass)>) {
    for (mut force, walker, grounded, mass) in &mut character_query {
        if let Some(acceleration) = walker.calculate_acceleration(grounded.is_grounded()) {
            let walking_force = acceleration * mass.0;
            force.0 += walking_force;
        }
    }
}
