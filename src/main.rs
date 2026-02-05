pub mod components;

use bevy::color::palettes::tailwind::ORANGE_500;
use bevy::mesh::VertexAttributeValues;
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy::window::{CursorGrabMode, CursorOptions};
use bevy_enhanced_input::prelude::*;
use bevy_flycam::FlyCam;
use bevy_sky_gradient::prelude::*;
use noiz::Noise;
use noiz::prelude::*;
use std::f32::consts::{FRAC_PI_2, FRAC_PI_8, PI};
use crate::components::PlayerPlugin;

#[derive(Component)]
struct Terrain;

fn toggle_wireframe(
    mut commands: Commands,
    landscapes_wireframe: Query<Entity, (With<Terrain>, With<Wireframe>)>,
    landscapes: Query<Entity, (With<Terrain>, Without<Wireframe>)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.just_pressed(KeyCode::Space) {
        for terrain in &landscapes {
            commands.entity(terrain).insert(Wireframe);
        }
        for terrain in landscapes_wireframe {
            commands.entity(terrain).remove::<Wireframe>();
        }
    }
}

fn main() {
    App::new()
        // .add_plugins(DefaultPlugins)
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
            WireframePlugin::default(),
        ))
        .add_systems(Startup, startup)
        .add_plugins(PlayerPlugin)
        .add_plugins(EnhancedInputPlugin)
        .add_systems(Update, toggle_wireframe)
        .add_plugins(SkyPlugin::default())
        .add_input_context::<FlyCam>() // All contexts should be registered.
        .add_observer(apply_movement)
        .add_observer(rotate)
        .add_observer(zoom)
        .add_observer(capture_cursor)
        .add_observer(release_cursor)
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    grab_cursor(&mut cursor_options, true);

    commands.spawn((
        // Add this tag to make the skybox follow the camera
        SkyboxMagnetTag,
        Camera3d::default(),
        FlyCam,
        Transform::from_xyz(-0.4, 25.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        actions!(FlyCam[
            (
                Action::<Movement>::new(),
                // Conditions and modifiers as components.
                DeadZone::default(), // Apply non-uniform normalization that works for both digital and analog inputs, otherwise diagonal movement will be faster.
                SmoothNudge::default(), // Make movement smooth and independent of the framerate. To only make it framerate-independent, use `DeltaScale`.
                Scale::splat(0.3), // Additionally multiply by a constant to achieve the desired speed.
                // Bindings are entities related to actions.
                // An action can have multiple bindings and will respond to any of them.
                Bindings::spawn((
                    // Bindings like WASD or sticks are very common,
                    // so we provide built-in `SpawnableList`s to assign all keys/axes at once.
                    // Cardinal::wasd_keys(),
                    Spatial::new(KeyCode::KeyW, KeyCode::KeyS, KeyCode::KeyA, KeyCode::KeyD, KeyCode::KeyE, KeyCode::KeyC),
                    Axial::left_stick(),
                )),
            ),
            (
                Action::<Rotate>::new(),
                Bindings::spawn((
                    // Bevy requires single entities to be wrapped in `Spawn`.
                    // You can attach modifiers to individual bindings as well.
                    Spawn((Binding::mouse_motion(), Scale::splat(0.1), Negate::all())),
                    Axial::right_stick().with((Scale::splat(2.0), Negate::x())),
                )),
            ),
            (
                Action::<Zoom>::new(),
                Scale::splat(0.1),
                Bindings::spawn((
                    // In Bevy, vertical scrolling maps to the Y axis,
                    // so we apply `SwizzleAxis` to map it to our 1-dimensional action.
                    Spawn((Binding::mouse_wheel(), SwizzleAxis::YXZ)),
                    Bidirectional::new(GamepadButton::DPadUp, GamepadButton::DPadDown),
                )),
            ),
            // For bindings we also have a macro similar to `children!`.
            (Action::<CaptureCursor>::new(), bindings![MouseButton::Left]),
            (Action::<ReleaseCursor>::new(), bindings![KeyCode::Escape]),
        ]),
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: light_consts::lux::OVERCAST_DAY,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.0),
            ..default()
        },
    ));

    const TERRAIN_HEIGHT: f32 = 50.0;

    let mut terrain: Mesh = Mesh::from(
        Plane3d::default()
            .mesh()
            .size(1000.0, 1000.0)
            .subdivisions(200),
    );

    if let Some(VertexAttributeValues::Float32x3(positions)) =
        terrain.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        // let noise = Noise::<BlendCellGradients<
        //     SimplexGrid,
        //     SimplecticBlend,
        //     QuickGradients,
        // >>::default();

        let noise = Noise::<
            LayeredNoise<
                Normed<f32>,
                Persistence,
                FractalLayers<Octave<MixCellGradients<OrthoGrid, Smoothstep, QuickGradients>>>,
            >,
        >::default();

        for pos in positions.iter_mut() {
            let value: f32 = noise.sample(Vec2::new(pos[0] / 100., pos[2] / 100.));
            pos[1] += value * TERRAIN_HEIGHT;
        }
    }

    terrain.compute_normals();

    commands.spawn((
        Mesh3d(meshes.add(terrain)),
        MeshMaterial3d(materials.add(Color::from(ORANGE_500))),
        Terrain,
    ));
}

fn apply_movement(movement: On<Fire<Movement>>, mut transforms: Query<&mut Transform>) {
    let mut transform = transforms.get_mut(movement.context).unwrap();

    // Move to the camera direction.
    let rotation = transform.rotation;

    // Movement consists of X and -Z components, so swap Y and Z with negation.
    // We could do it with modifiers, but it would be weird for an action to return
    // a `Vec3` like this, so we doing it inside the function.
    // let mut velocity = movement.value.extend(0.0).xzy();
    let velocity = movement.value;

    // velocity.z = -velocity.z;

    transform.translation += rotation * velocity
}

fn rotate(
    rotate: On<Fire<Rotate>>,
    mut transforms: Query<&mut Transform>,
    cursor_options: Single<&CursorOptions>,
) {
    if cursor_options.visible {
        return;
    }

    let mut transform = transforms.get_mut(rotate.context).unwrap();
    let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);

    yaw += rotate.value.x.to_radians();
    pitch -= rotate.value.y.to_radians();

    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
}

fn zoom(zoom: On<Fire<Zoom>>, mut projections: Query<&mut Projection>) {
    let mut projection = projections.get_mut(zoom.context).unwrap();
    let Projection::Perspective(projection) = &mut *projection else {
        panic!("camera should be perspective");
    };
    projection.fov = (projection.fov - zoom.value).clamp(FRAC_PI_8, FRAC_PI_2);
}

fn capture_cursor(
    _on: On<Complete<CaptureCursor>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    grab_cursor(&mut cursor_options, true);
}

fn release_cursor(
    _on: On<Complete<ReleaseCursor>>,
    mut cursor_options: Single<&mut CursorOptions>,
) {
    grab_cursor(&mut cursor_options, false);
}

fn grab_cursor(cursor_options: &mut CursorOptions, grab: bool) {
    cursor_options.grab_mode = if grab {
        CursorGrabMode::Confined
    } else {
        CursorGrabMode::None
    };
    cursor_options.visible = !grab;
}

// All actions should implement the `InputAction` trait.
// It can be done manually, but we provide a derive for convenience.
// The only attribute is `action_output`, which defines the output type.
struct Movement;

impl InputAction for Movement {
    type Output = Vec3;
}

#[derive(InputAction)]
#[action_output(Vec2)]
struct Rotate;

#[derive(InputAction)]
#[action_output(f32)]
struct Zoom;

#[derive(InputAction)]
#[action_output(bool)]
struct CaptureCursor;

#[derive(InputAction)]
#[action_output(bool)]
struct ReleaseCursor;
