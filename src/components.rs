use bevy::prelude::*;

#[derive(Component)]
#[require(SceneRoot, Transform)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_player)
            .add_systems(Update, move_character);
    }
}

fn move_character(
    mut player: Query<&mut Transform, With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if input.pressed(KeyCode::KeyU) {
        for mut transform in &mut player {
            transform.translation.y += 0.2;
        }
    }

    if input.pressed(KeyCode::KeyJ) {
        for mut transform in &mut player {
            transform.translation.y -= 0.2;
        }
    }
}

fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Player,
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset("Ranger.gltf"))),
        Transform::from_xyz(0.0, 10.0, 0.0),
    ));
}
