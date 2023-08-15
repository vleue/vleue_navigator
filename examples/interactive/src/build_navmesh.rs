use bevy::{prelude::*, render::primitives::Aabb};
use bevy_pathmesh::{
    updater::{NavMeshStatus, NavmeshUpdaterPlugin},
    PathMesh,
};

use crate::{
    ui::{Slider, UiButton, UiInfo},
    Agent, Obstacle, Path, Target, HANDLE_NAVMESH_MESH, HANDLE_NAVMESH_WIREFRAME,
};

pub struct BuilderPlugin;

impl Plugin for BuilderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(NavmeshUpdaterPlugin::<Obstacle, Aabb>::default())
            .add_systems(Update, update);
    }
}

fn update(
    mut commands: Commands,
    obstacles: Query<(Ref<GlobalTransform>, &Aabb), With<Obstacle>>,
    mut meshes: ResMut<Assets<Mesh>>,
    pathmeshes: Res<Assets<PathMesh>>,
    mut agents: Query<(Entity, &mut Agent)>,
    targets: Query<&Transform, With<Target>>,
    mut text_info: Query<(&mut Text, &UiInfo)>,
    mut sliders: Query<(&mut Slider, &UiButton)>,
    navmeshes: Query<(&Handle<PathMesh>, Ref<NavMeshStatus>)>,
) {
    for (handle, status) in &navmeshes {
        if status.is_changed() {
            let pathmesh = pathmeshes.get(handle).unwrap();

            for (agent_entity, mut agent) in &mut agents {
                commands.entity(agent_entity).remove::<Path>();
                if let Some(target_entity) = agent.target {
                    if let Ok(target_transform) = targets.get(target_entity) {
                        if !pathmesh.transformed_is_in_mesh(target_transform.translation) {
                            commands.entity(target_entity).despawn_recursive();
                            agent.target = None;
                        }
                    }
                }
            }

            for (mut text, info) in &mut text_info {
                match info {
                    UiInfo::PolygonCount => {
                        text.sections[1].value = pathmesh.get().polygons.len().to_string();
                    }
                    UiInfo::ObstacleCount => {
                        text.sections[1].value = obstacles.iter().count().to_string();
                    }
                    UiInfo::Simplification => {
                        text.sections[0].style.color = Color::WHITE;
                    }
                    _ => (),
                }
            }
            for (mut slider, button) in &mut sliders {
                match button {
                    UiButton::Simplification => slider.line_color = Color::GREEN,
                    _ => (),
                }
            }

            meshes.set_untracked(HANDLE_NAVMESH_WIREFRAME, pathmesh.to_wireframe_mesh());
            meshes.set_untracked(HANDLE_NAVMESH_MESH, pathmesh.to_mesh());
        }
        match *status {
            NavMeshStatus::Failed => {
                for (mut text, info) in &mut text_info {
                    match info {
                        UiInfo::ObstacleCount => {
                            text.sections[1].value = obstacles.iter().count().to_string();
                        }
                        UiInfo::Simplification => {
                            text.sections[0].style.color = Color::RED;
                        }
                        _ => (),
                    }
                }
                for (mut slider, button) in &mut sliders {
                    match button {
                        UiButton::Simplification => slider.line_color = Color::RED,
                        _ => (),
                    }
                }
            }
            _ => (),
        }
    }
}
