use bevy::prelude::*;

use crate::Obstacle;

pub struct HelperPlugin;

impl Plugin for HelperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, block_moving_obstacles_out);
    }
}

fn block_moving_obstacles_out(mut obstacle_transforms: Query<&mut Transform, With<Obstacle>>) {
    for mut transform in &mut obstacle_transforms {
        if transform.translation.x < -4.4 {
            transform.translation.x = -4.4;
        }
        if transform.translation.x > 4.4 {
            transform.translation.x = 4.4;
        }
        if transform.translation.z < -4.4 {
            transform.translation.z = -4.4;
        }
        if transform.translation.z > 4.4 {
            transform.translation.z = 4.4;
        }
    }
}
