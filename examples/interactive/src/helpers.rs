use bevy::prelude::*;

use crate::{BOARD_LIMIT, Obstacle};

pub struct HelperPlugin;

impl Plugin for HelperPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, block_moving_obstacles_out);
    }
}

fn block_moving_obstacles_out(mut obstacle_transforms: Query<&mut Transform, With<Obstacle>>) {
    for mut transform in &mut obstacle_transforms {
        if transform.translation.x < -BOARD_LIMIT {
            transform.translation.x = -BOARD_LIMIT;
        }
        if transform.translation.x > BOARD_LIMIT {
            transform.translation.x = BOARD_LIMIT;
        }
        if transform.translation.z < -BOARD_LIMIT {
            transform.translation.z = -BOARD_LIMIT;
        }
        if transform.translation.z > BOARD_LIMIT {
            transform.translation.z = BOARD_LIMIT;
        }
    }
}
