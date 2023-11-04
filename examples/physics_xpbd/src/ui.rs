use bevy::{prelude::*, render::view::RenderLayers, ui::RelativeCursorPosition};
use bevy_pathmesh::{updater::NavMeshSettings, PathMesh};
use bevy_vector_shapes::{
    prelude::ShapePainter,
    shapes::{Cap, DiscPainter, LinePainter},
};
use rand::Rng;

use crate::{Agent, Obstacle, HANDLE_AGENT_MATERIAL, HANDLE_AGENT_MESH};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (button_system, sliders, show_settings));
    }
}

fn setup(mut commands: Commands) {
    let info_text_size = 20.0;
    let detail_text_size = 30.0;
    commands
        .spawn(NodeBundle {
            style: Style {
                justify_content: JustifyContent::SpaceBetween,
                width: Val::Percent(100.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        margin: UiRect::all(Val::Px(20.0)),

                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(220.0),
                                    height: Val::Px(40.0),
                                    border: UiRect::all(Val::Px(5.0)),
                                    margin: UiRect::all(Val::Px(10.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                border_color: BorderColor(Color::BLUE),
                                background_color: Color::GRAY.into(),
                                ..default()
                            },
                            UiButton::ClearObstacles,
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle::from_section(
                                "Clear Obstacles",
                                TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            ));
                        });

                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(220.0),
                                    height: Val::Px(40.0),
                                    border: UiRect::all(Val::Px(5.0)),
                                    margin: UiRect::all(Val::Px(10.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                border_color: BorderColor(Color::GREEN),
                                background_color: Color::GRAY.into(),
                                ..default()
                            },
                            UiButton::SpawnAgent,
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle::from_section(
                                "Spawn Agent",
                                TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            ));
                        });
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(220.0),
                                    height: Val::Px(40.0),
                                    border: UiRect::all(Val::Px(5.0)),
                                    margin: UiRect::all(Val::Px(10.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                border_color: BorderColor(Color::BLUE),
                                background_color: Color::GRAY.into(),
                                ..default()
                            },
                            UiButton::ClearAgents,
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle::from_section(
                                "Clear Agents",
                                TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            ));
                        });
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexEnd,
                        margin: UiRect::all(Val::Px(20.0)),

                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    {
                        parent.spawn(TextBundle::from_sections(vec![
                            TextSection {
                                value: "simplification\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                            TextSection {
                                value: "Points that impact less than this area\nin an obstacle will be ignored\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size * 0.6,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                            TextSection {
                                value: "Putting a value too high can cause the\nnavmesh generation to fail".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size * 0.6,
                                    color: Color::rgb(0.9, 0.3, 0.3),
                                    ..default()
                                },
                            },
                        ])
                        .with_text_alignment(TextAlignment::Right)
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        }));

                        parent.spawn(NodeBundle::default()).with_children(|parent| {
                            parent.spawn((
                                UiButton::Simplification,
                                ButtonBundle {
                                    background_color: BackgroundColor(Color::NONE),
                                    style: Style {
                                        width: Val::Px(200.0),
                                        height: Val::Px(30.0),
                                        ..default()
                                    },
                                    ..default()
                                },
                                Slider {
                                    value: 0.0,
                                    line_color: Color::GREEN,
                                    border_width: 5.0,
                                    z: 10.0,
                                },
                                RelativeCursorPosition::default(),
                            ));
                            parent.spawn((
                                TextBundle::from_sections(vec![TextSection {
                                    value: "0.001".to_string(),
                                    style: TextStyle {
                                        font_size: info_text_size,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..default()
                                    },
                                }])
                                .with_text_alignment(TextAlignment::Right)
                                .with_style(Style {
                                    margin: UiRect::horizontal(Val::Px(10.0)),
                                    ..default()
                                }),
                                UiInfo::Simplification,
                            ));
                        });
                    }
                    {
                        parent.spawn(TextBundle::from_sections(vec![
                                TextSection {
                                value: "merge steps\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                            TextSection {
                                value: "Number of iteration through polygons\nto try and merge them".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size * 0.6,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            }
                        ])
                        .with_text_alignment(TextAlignment::Right)
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        }));

                        parent.spawn(NodeBundle::default()).with_children(|parent| {
                            parent.spawn((
                                UiButton::MergeSteps,
                                ButtonBundle {
                                    background_color: BackgroundColor(Color::NONE),
                                    style: Style {
                                        width: Val::Px(200.0),
                                        height: Val::Px(30.0),
                                        ..default()
                                    },
                                    ..default()
                                },
                                Slider {
                                    value: 0.0,
                                    line_color: Color::GREEN,
                                    border_width: 5.0,
                                    z: 10.0,
                                },
                                RelativeCursorPosition::default(),
                            ));
                            parent.spawn((
                                TextBundle::from_sections(vec![TextSection {
                                    value: "0".to_string(),
                                    style: TextStyle {
                                        font_size: info_text_size,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..default()
                                    },
                                }])
                                .with_text_alignment(TextAlignment::Right)
                                .with_style(Style {
                                    margin: UiRect::horizontal(Val::Px(10.0)),
                                    ..default()
                                }),
                                UiInfo::MergeSteps,
                            ));
                        });
                    }
                    {
                        parent.spawn(TextBundle::from_sections(vec![
                            TextSection {
                                value: "unit radius\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                        ])
                        .with_text_alignment(TextAlignment::Right)
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        }));

                        parent.spawn(NodeBundle::default()).with_children(|parent| {
                            parent.spawn((
                                UiButton::UnitRadius,
                                ButtonBundle {
                                    background_color: BackgroundColor(Color::NONE),
                                    style: Style {
                                        width: Val::Px(200.0),
                                        height: Val::Px(30.0),
                                        ..default()
                                    },
                                    ..default()
                                },
                                Slider {
                                    value: 0.0,
                                    line_color: Color::GREEN,
                                    border_width: 5.0,
                                    z: 10.0,
                                },
                                RelativeCursorPosition::default(),
                            ));
                            parent.spawn((
                                TextBundle::from_sections(vec![TextSection {
                                    value: "0.001".to_string(),
                                    style: TextStyle {
                                        font_size: info_text_size,
                                        color: Color::rgb(0.9, 0.9, 0.9),
                                        ..default()
                                    },
                                }])
                                .with_text_alignment(TextAlignment::Right)
                                .with_style(Style {
                                    margin: UiRect::horizontal(Val::Px(10.0)),
                                    ..default()
                                }),
                                UiInfo::UnitRadius,
                            ));
                        });
                    }

                    parent.spawn(NodeBundle{
                        style: Style {
                            height: Val::Px(40.0),
                            ..default()
                        },
                        ..default()
                    });
                    parent.spawn((
                        TextBundle::from_sections(vec![
                            TextSection {
                                value: "number of polygons\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                            TextSection {
                                value: "0".to_string(),
                                style: TextStyle {
                                    font_size: detail_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                        ])
                        .with_text_alignment(TextAlignment::Right)
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        }),
                        UiInfo::PolygonCount,
                    ));
                    parent.spawn((
                        TextBundle::from_sections(vec![
                            TextSection {
                                value: "number of obstacles\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                            TextSection {
                                value: "0".to_string(),
                                style: TextStyle {
                                    font_size: detail_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                        ])
                        .with_text_alignment(TextAlignment::Right)
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        }),
                        UiInfo::ObstacleCount,
                    ));
                    parent.spawn((
                        TextBundle::from_sections(vec![
                            TextSection {
                                value: "number of agents\n".to_string(),
                                style: TextStyle {
                                    font_size: info_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                            TextSection {
                                value: "0".to_string(),
                                style: TextStyle {
                                    font_size: detail_text_size,
                                    color: Color::rgb(0.9, 0.9, 0.9),
                                    ..default()
                                },
                            },
                        ])
                        .with_text_alignment(TextAlignment::Right)
                        .with_style(Style {
                            margin: UiRect::all(Val::Px(10.0)),
                            ..default()
                        }),
                        UiInfo::AgentCount,
                    ));
                });
        });
}

#[derive(Component)]
pub enum UiButton {
    ClearObstacles,
    SpawnAgent,
    ClearAgents,
    Simplification,
    MergeSteps,
    UnitRadius,
}

#[derive(Component)]
pub enum UiInfo {
    PolygonCount,
    ObstacleCount,
    AgentCount,
    Simplification,
    MergeSteps,
    UnitRadius,
}

fn button_system(
    mut commands: Commands,
    mut interaction_query: Query<
        (Ref<Interaction>, &UiButton, Option<&RelativeCursorPosition>),
        With<Button>,
    >,
    obstacles: Query<Entity, With<Obstacle>>,
    agents: Query<(Entity, &Agent)>,
    pathmeshes: Res<Assets<PathMesh>>,
    mut text_info: Query<(&mut Text, &UiInfo)>,
    mut settings: Query<&mut NavMeshSettings>,
    navmesh: Query<&Handle<PathMesh>>,
) {
    let mut settings = settings.single_mut();

    for (interaction, button, relative_position) in &mut interaction_query {
        match (*interaction, interaction.is_changed(), button) {
            (Interaction::Pressed, true, UiButton::ClearObstacles) => {
                for entity in &obstacles {
                    commands.entity(entity).despawn_recursive();
                }
            }

            (Interaction::Pressed, true, UiButton::SpawnAgent) => {
                let navmesh = pathmeshes.get(navmesh.single()).unwrap();
                let mut x;
                let mut z;
                loop {
                    x = rand::thread_rng().gen_range(-4.95..4.95);
                    z = rand::thread_rng().gen_range(-4.95..4.95);

                    if navmesh.transformed_is_in_mesh(Vec3::new(x, 0.0, z)) {
                        break;
                    }
                }

                commands.spawn((
                    PbrBundle {
                        mesh: HANDLE_AGENT_MESH,
                        material: HANDLE_AGENT_MATERIAL,
                        transform: Transform::from_xyz(x, 0.0, z),
                        ..Default::default()
                    },
                    Agent { target: None },
                    RenderLayers::layer(1),
                ));
                for (mut text, info) in &mut text_info {
                    match info {
                        UiInfo::AgentCount => {
                            text.sections[1].value = format!("{}", agents.iter().count() + 1)
                        }
                        _ => (),
                    }
                }
            }
            (Interaction::Pressed, true, UiButton::ClearAgents) => {
                for (entity, target) in &agents {
                    if let Some(target_entity) = target.target {
                        commands.entity(target_entity).despawn_recursive();
                    }
                    commands.entity(entity).despawn_recursive();
                }
                for (mut text, info) in &mut text_info {
                    match info {
                        UiInfo::AgentCount => text.sections[1].value = "0".to_string(),
                        _ => (),
                    }
                }
            }
            (Interaction::Pressed, _, UiButton::Simplification) => {
                let mut value = ((relative_position.unwrap().normalized.unwrap().x - 0.5)
                    / SLIDER_WIDTH_RATIO
                    + 0.5)
                    .clamp(0.0, 1.0)
                    / 25.0;
                if value < 0.001 {
                    value = 0.0;
                }
                if settings.simplify != value {
                    settings.simplify = value;
                }
            }
            (Interaction::Pressed, _, UiButton::MergeSteps) => {
                let value = (((relative_position.unwrap().normalized.unwrap().x - 0.5)
                    / SLIDER_WIDTH_RATIO
                    + 0.5)
                    .clamp(0.0, 1.0)
                    * 4.0)
                    .round() as usize;
                if settings.merge_steps != value {
                    settings.merge_steps = value;
                }
            }
            (Interaction::Pressed, _, UiButton::UnitRadius) => {
                let mut value = ((relative_position.unwrap().normalized.unwrap().x - 0.5)
                    / SLIDER_WIDTH_RATIO
                    + 0.5)
                    .clamp(0.0, 1.0)
                    * 2.0;
                if value < 0.001 {
                    value = 0.0;
                }
                if settings.unit_radius != value {
                    settings.unit_radius = value;
                }
            }

            _ => (),
        }
    }
}

#[derive(Component)]
pub struct Slider {
    pub value: f32,
    pub line_color: Color,
    pub border_width: f32,
    pub z: f32,
}

const SLIDER_WIDTH_RATIO: f32 = 0.8;

fn sliders(
    sliders: Query<(&Node, &GlobalTransform, &Slider)>,
    window: Query<&Window>,
    mut painter: ShapePainter,
) {
    let window = window.single();
    for (node, global_transform, slider) in &sliders {
        let width = node.size().x * SLIDER_WIDTH_RATIO;
        painter.set_translation(
            global_transform.translation() * Vec3::new(1.0, -1.0, 1.0)
                - Vec3::new(window.width() / 2.0, -window.height() / 2.0, 0.0),
        );
        painter.translate(Vec3::new(0.0, 0.0, slider.z));
        painter.translate(Vec3::new(-0.5, 0.0, 0.0) * width);

        painter.cap = Cap::Round;
        painter.thickness = slider.border_width;

        painter.color = Color::WHITE;
        painter.line(
            Vec3::new(slider.value, 0.0, 1.0) * width,
            Vec3::new(1.0, 0.0, 1.0) * width,
        );

        if slider.value != 0.0 {
            painter.color = slider.line_color;
            painter.line(
                Vec3::new(0.0, 0.0, 1.0) * width,
                Vec3::new(slider.value, 0.0, 1.0) * width,
            );
        }

        painter.translate(Vec3::new(slider.value, 0.0, 0.0) * width);
        painter.hollow = true;
        painter.circle(slider.border_width * 2.0);
    }
}

fn show_settings(
    mut text_info: Query<(&mut Text, &UiInfo)>,
    mut sliders: Query<(&mut Slider, &UiButton)>,
    settings: Query<Ref<NavMeshSettings>>,
) {
    let settings = settings.single();
    if settings.is_changed() {
        for (mut text, info) in &mut text_info {
            match info {
                UiInfo::Simplification => {
                    text.sections[0].value = format!("{:.3}", settings.simplify)
                }
                UiInfo::MergeSteps => {
                    text.sections[0].value = format!("{}    ", settings.merge_steps)
                }
                UiInfo::UnitRadius => {
                    text.sections[0].value = format!("{:.3}", settings.unit_radius)
                }
                _ => (),
            }
        }
        for (mut slider, button) in &mut sliders {
            match button {
                UiButton::Simplification => slider.value = settings.simplify * 25.0,
                UiButton::MergeSteps => slider.value = settings.merge_steps as f32 / 4.0,
                UiButton::UnitRadius => slider.value = settings.unit_radius / 2.0,
                _ => (),
            }
        }
    }
}
