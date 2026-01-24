use bevy::color::palettes::basic::WHITE;
use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::prelude::*;
use bevy::render::RenderPlugin;
use bevy::render::render_resource::WgpuFeatures;
use bevy::render::settings::{RenderCreation, WgpuSettings};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use std::f32::consts::PI;

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
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    // WARN this is a native only feature. It will not work with webgl or webgpu
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
            // You need to add this plugin to enable wireframe rendering
            WireframePlugin::default(),
            PanOrbitCameraPlugin,
        ))
        .add_systems(Startup, startup)
        .add_systems(Update, toggle_wireframe)
        .run();
}

fn startup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(materials.add(Color::from(WHITE))),
        Terrain,
    ));

    commands.spawn((
        Transform::from_xyz(0.0, 20.0, 75.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        PanOrbitCamera::default(),
    ));
}
