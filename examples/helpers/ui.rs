use bevy::{color::palettes, diagnostic::DiagnosticsStore, prelude::*};
use vleue_navigator::prelude::*;

#[derive(Component)]
pub enum UiSettings {
    Simplify,
    MergeSteps,
    AgentRadius,
    Cache,
}

#[derive(Component)]
pub enum UiSettingsButtons {
    SimplifyInc,
    SimplifyDec,
    MergeStepsInc,
    MergeStepsDec,
    AgentRadiusInc,
    AgentRadiusDec,
    ToggleCache,
}

#[derive(Resource, Default)]
pub struct ExampleSettings {
    pub cache_enabled: bool,
}

fn button(text: &str, action: UiSettingsButtons, parent: &mut ChildBuilder) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    margin: UiRect::all(Val::Px(5.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    justify_content: JustifyContent::Center,
                    height: Val::Px(30.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                border_color: BorderColor(palettes::tailwind::GRAY_500.into()),
                border_radius: BorderRadius::MAX,
                background_color: palettes::tailwind::GRAY_700.into(),
                ..default()
            },
            action,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font_size: 20.0,
                    ..default()
                },
            ));
        });
}

pub fn setup_settings<const WITH_CACHE: bool>(mut commands: Commands) {
    commands.init_resource::<ExampleSettings>();
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    right: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                border_radius: BorderRadius {
                    top_left: Val::Px(0.),
                    top_right: Val::Px(0.),
                    bottom_left: Val::Px(20.0),
                    bottom_right: Val::Px(0.),
                },
                background_color: BackgroundColor(
                    palettes::tailwind::GRAY_900.with_alpha(0.8).into(),
                ),
                ..default()
            },
            Ui,
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle { ..default() })
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle {
                            text: Text::from_sections(
                                [("Simplify: ", 30.0), ("{}", 30.0)].into_iter().map(
                                    |(text, font_size): (&str, f32)| {
                                        TextSection::new(
                                            text,
                                            TextStyle {
                                                font_size,
                                                ..default()
                                            },
                                        )
                                    },
                                ),
                            ),
                            style: Style {
                                margin: UiRect::all(Val::Px(12.0)),
                                ..default()
                            },
                            ..default()
                        }
                        .with_text_justify(JustifyText::Right),
                        UiSettings::Simplify,
                    ));
                    button(" - ", UiSettingsButtons::SimplifyDec, parent);
                    button(" + ", UiSettingsButtons::SimplifyInc, parent);
                });
            parent
                .spawn(NodeBundle { ..default() })
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle {
                            text: Text::from_sections(
                                [("Merge Steps: ", 30.0), ("{}", 30.0)].into_iter().map(
                                    |(text, font_size): (&str, f32)| {
                                        TextSection::new(
                                            text,
                                            TextStyle {
                                                font_size,
                                                ..default()
                                            },
                                        )
                                    },
                                ),
                            ),
                            style: Style {
                                margin: UiRect::all(Val::Px(12.0)),
                                ..default()
                            },
                            ..default()
                        }
                        .with_text_justify(JustifyText::Right),
                        UiSettings::MergeSteps,
                    ));
                    button(" - ", UiSettingsButtons::MergeStepsDec, parent);
                    button(" + ", UiSettingsButtons::MergeStepsInc, parent);
                });
            parent
                .spawn(NodeBundle { ..default() })
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle {
                            text: Text::from_sections(
                                [("Agent Radius: ", 30.0), ("{}", 30.0)].into_iter().map(
                                    |(text, font_size): (&str, f32)| {
                                        TextSection::new(
                                            text,
                                            TextStyle {
                                                font_size,
                                                ..default()
                                            },
                                        )
                                    },
                                ),
                            ),
                            style: Style {
                                margin: UiRect::all(Val::Px(12.0)),
                                ..default()
                            },
                            ..default()
                        }
                        .with_text_justify(JustifyText::Right),
                        UiSettings::AgentRadius,
                    ));
                    button(" - ", UiSettingsButtons::AgentRadiusDec, parent);
                    button(" + ", UiSettingsButtons::AgentRadiusInc, parent);
                });
            if WITH_CACHE {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                margin: UiRect::px(30.0, 30.0, 10.0, 30.0),
                                border: UiRect::all(Val::Px(1.0)),
                                justify_content: JustifyContent::Center,
                                height: Val::Px(30.0),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            border_color: BorderColor(palettes::tailwind::GRAY_500.into()),
                            border_radius: BorderRadius::all(Val::Px(10.0)),
                            image: UiImage::default()
                                .with_color(palettes::tailwind::GRAY_700.into()),
                            ..default()
                        },
                        UiSettingsButtons::ToggleCache,
                        UiSettings::Cache,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Cache",
                            TextStyle {
                                font_size: 20.0,
                                ..default()
                            },
                        ));
                    });
            }
        });
}

pub fn display_settings(
    settings: Query<Ref<NavMeshSettings>>,
    example_settings: Res<ExampleSettings>,
    mut texts: Query<(&mut Text, &UiSettings)>,
    mut buttons: Query<(&mut BackgroundColor, &UiSettings), With<Button>>,
) {
    let settings = settings.single();
    if settings.is_changed() {
        for (mut text, param) in &mut texts {
            match param {
                UiSettings::Simplify => {
                    text.sections[1].value = format!("{:.2}", settings.simplify)
                }
                UiSettings::MergeSteps => {
                    text.sections[1].value = format!("{}", settings.merge_steps)
                }
                UiSettings::AgentRadius => {
                    text.sections[1].value = format!("{}", settings.agent_radius)
                }
                UiSettings::Cache => (),
            }
        }
    }
    if example_settings.is_changed() {
        for (mut color, param) in &mut buttons {
            match param {
                UiSettings::Simplify => (),
                UiSettings::MergeSteps => (),
                UiSettings::AgentRadius => (),
                UiSettings::Cache => {
                    *color = if example_settings.cache_enabled {
                        palettes::tailwind::GREEN_400.into()
                    } else {
                        palettes::tailwind::RED_600.into()
                    }
                }
            }
        }
    }
}

pub fn update_settings<const STEP: u32>(
    mut interaction_query: Query<
        (&Interaction, &UiSettingsButtons, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: Query<&mut NavMeshSettings>,
    mut example_settings: ResMut<ExampleSettings>,
) {
    for (interaction, button, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let mut settings = settings.single_mut();
                match *button {
                    UiSettingsButtons::SimplifyDec => {
                        settings.simplify = (settings.simplify - STEP as f32 / 1000.0).max(0.0);
                    }
                    UiSettingsButtons::SimplifyInc => {
                        settings.simplify = (settings.simplify + STEP as f32 / 1000.0).min(10.0);
                    }
                    UiSettingsButtons::MergeStepsDec => {
                        settings.merge_steps = settings.merge_steps.saturating_sub(1);
                    }
                    UiSettingsButtons::MergeStepsInc => {
                        settings.merge_steps = (settings.merge_steps + 1).min(5);
                    }
                    UiSettingsButtons::AgentRadiusDec => {
                        settings.agent_radius = (settings.agent_radius - 0.5).max(0.0);
                    }
                    UiSettingsButtons::AgentRadiusInc => {
                        settings.agent_radius = (settings.agent_radius + 0.5).min(10.0);
                    }
                    UiSettingsButtons::ToggleCache => {
                        example_settings.cache_enabled = !example_settings.cache_enabled;
                    }
                }
            }
            Interaction::Hovered => {
                if !matches!(button, UiSettingsButtons::ToggleCache) {
                    *color = palettes::tailwind::GRAY_600.into();
                }
            }
            Interaction::None => {
                if !matches!(button, UiSettingsButtons::ToggleCache) {
                    *color = palettes::tailwind::GRAY_700.into();
                }
            }
        }
    }
}

pub fn setup_stats<const INTERACTIVE: bool>(mut commands: Commands) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    flex_direction: FlexDirection::Column,
                    min_width: Val::Px(300.0),
                    ..default()
                },
                border_radius: BorderRadius {
                    top_left: Val::Px(0.),
                    top_right: Val::Px(0.),
                    bottom_right: Val::Px(20.0),
                    bottom_left: Val::Px(0.),
                },
                background_color: BackgroundColor(
                    palettes::tailwind::GRAY_900.with_alpha(0.8).into(),
                ),
                ..default()
            },
            Ui,
        ))
        .with_children(|parent| {
            let mut text = vec![
                ("Status: ", 30.0),
                ("{}", 30.0),
                ("\nObstacles: ", 30.0),
                ("{}", 30.0),
                ("\nPolygons: ", 30.0),
                ("{}", 30.0),
                ("\nBuild Duration: ", 30.0),
                ("{}", 30.0),
                ("ms", 30.0),
            ];
            if INTERACTIVE {
                text.push(("\n\nClick to add an obstacle", 25.0));
                text.push(("\nPress spacebar to reset", 25.0));
            }
            parent.spawn((
                TextBundle {
                    text: Text::from_sections(text.into_iter().map(
                        |(text, font_size): (&str, f32)| {
                            TextSection::new(
                                text,
                                TextStyle {
                                    font_size,
                                    ..default()
                                },
                            )
                        },
                    )),
                    style: Style {
                        margin: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    ..default()
                },
                UiStats,
            ));
        });
}

pub fn update_stats<T: Component>(
    mut text: Query<&mut Text, With<UiStats>>,
    obstacles: Query<&T>,
    navmesh: Query<(Ref<NavMeshStatus>, &Handle<NavMesh>)>,
    navmeshes: Res<Assets<NavMesh>>,
    diagnostics: Res<DiagnosticsStore>,
) {
    let (status, handle) = navmesh.single();

    if !status.is_changed() && !status.is_added() {
        return;
    }

    let mut text = text.single_mut();
    text.sections[1].value = format!("{:?}", *status);
    text.sections[1].style.color = match *status {
        NavMeshStatus::Building => palettes::tailwind::AMBER_500.into(),
        NavMeshStatus::Built => palettes::tailwind::GREEN_400.into(),
        NavMeshStatus::Failed => palettes::tailwind::RED_600.into(),
        NavMeshStatus::Cancelled => palettes::tailwind::AMBER_500.into(),
        NavMeshStatus::Invalid => palettes::tailwind::RED_800.into(),
    };
    text.sections[3].value = format!("{}", obstacles.iter().len());
    text.sections[5].value = format!(
        "{}",
        navmeshes
            .get(handle)
            .map(|nm| nm
                .get()
                .layers
                .iter()
                .map(|l| l.polygons.len())
                .sum::<usize>())
            .unwrap_or_default()
    );
    text.sections[7].value = format!(
        "{:.3}",
        diagnostics
            .get(&NAVMESH_BUILD_DURATION)
            .unwrap()
            .smoothed()
            .unwrap_or_default()
            * 1000.0
    );
}

#[derive(Component)]
pub struct UiStats;

#[derive(Component)]
pub struct Ui;

#[allow(dead_code)]
pub fn toggle_ui(
    mut stats: Query<&mut Visibility, With<Ui>>,
    mut entered: EventReader<CursorEntered>,
    mut left: EventReader<CursorLeft>,
) {
    for _ in entered.read() {
        for mut visibility in &mut stats {
            *visibility = Visibility::Visible
        }
    }
    for _ in left.read() {
        for mut visibility in &mut stats {
            *visibility = Visibility::Hidden
        }
    }
}
