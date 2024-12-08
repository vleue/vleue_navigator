use bevy::{color::palettes, diagnostic::DiagnosticsStore, prelude::*};
use vleue_navigator::prelude::*;

#[derive(Component)]
pub enum UiSettings {
    Simplify,
    MergeSteps,
    AgentRadius,
    AgentRadiusOuter,
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
    AgentRadiusOuterToggle,
    ToggleCache,
}

#[derive(Resource, Default)]
pub struct ExampleSettings {
    pub cache_enabled: bool,
}

fn button(text: &str, action: UiSettingsButtons, parent: &mut ChildBuilder) {
    parent
        .spawn((
            Node {
                margin: UiRect::all(Val::Px(5.0)),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                height: Val::Px(30.0),
                align_items: AlignItems::Center,
                ..default()
            },
            Button,
            BorderColor(palettes::tailwind::GRAY_500.into()),
            BorderRadius::MAX,
            BackgroundColor(palettes::tailwind::GRAY_700.into()),
            action,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text(text.to_string()),
                TextFont {
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
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BorderRadius {
                top_left: Val::Px(0.),
                top_right: Val::Px(0.),
                bottom_left: Val::Px(20.0),
                bottom_right: Val::Px(0.),
            },
            BackgroundColor(palettes::tailwind::GRAY_900.with_alpha(0.8).into()),
            Ui,
        ))
        .with_children(|parent| {
            parent.spawn(Node { ..default() }).with_children(|parent| {
                parent
                    .spawn((
                        Text("Simplify: ".to_string()),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextLayout {
                            justify: JustifyText::Right,
                            ..default()
                        },
                        UiSettings::Simplify,
                    ))
                    .with_child((
                        TextSpan::default(),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                    ));
                button(" - ", UiSettingsButtons::SimplifyDec, parent);
                button(" + ", UiSettingsButtons::SimplifyInc, parent);
            });
            parent.spawn(Node { ..default() }).with_children(|parent| {
                parent
                    .spawn((
                        Text("Merge Steps: ".to_string()),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextLayout {
                            justify: JustifyText::Right,
                            ..default()
                        },
                        UiSettings::MergeSteps,
                    ))
                    .with_child((
                        TextSpan::default(),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                    ));
                button(" - ", UiSettingsButtons::MergeStepsDec, parent);
                button(" + ", UiSettingsButtons::MergeStepsInc, parent);
            });
            parent.spawn(Node { ..default() }).with_children(|parent| {
                parent
                    .spawn((
                        Text("Agent Radius: ".to_string()),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextLayout {
                            justify: JustifyText::Right,
                            ..default()
                        },
                        UiSettings::AgentRadius,
                    ))
                    .with_child((
                        TextSpan::default(),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                    ));
                button(" - ", UiSettingsButtons::AgentRadiusDec, parent);
                button(" + ", UiSettingsButtons::AgentRadiusInc, parent);
            });
            parent
                .spawn((
                    Node {
                        margin: UiRect::px(30.0, 30.0, 10.0, 30.0),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        height: Val::Px(30.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Button,
                    BorderColor(palettes::tailwind::GRAY_500.into()),
                    BorderRadius::all(Val::Px(10.0)),
                    BackgroundColor(palettes::tailwind::GRAY_700.into()),
                    UiSettingsButtons::AgentRadiusOuterToggle,
                    UiSettings::AgentRadiusOuter,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text("Agent Radius on Outer Edges".to_string()),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                    ));
                });
            if WITH_CACHE {
                parent
                    .spawn((
                        Node {
                            margin: UiRect::px(30.0, 30.0, 10.0, 30.0),
                            border: UiRect::all(Val::Px(1.0)),
                            justify_content: JustifyContent::Center,
                            height: Val::Px(30.0),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        Button,
                        BorderColor(palettes::tailwind::GRAY_500.into()),
                        BorderRadius::all(Val::Px(10.0)),
                        UiSettingsButtons::ToggleCache,
                        UiSettings::Cache,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Text("Cache".to_string()),
                            TextFont {
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
    mut texts: Query<(Entity, &UiSettings)>,
    mut buttons: Query<(&mut BackgroundColor, &UiSettings), With<Button>>,
    mut text_writer: TextUiWriter,
) {
    let settings = settings.single();
    if settings.is_changed() {
        for (text, param) in &mut texts {
            match param {
                UiSettings::Simplify => {
                    *text_writer.text(text, 1) = format!("{:.2}", settings.simplify);
                    // text.sections[1].value = format!("{:.2}", settings.simplify)
                }
                UiSettings::MergeSteps => {
                    *text_writer.text(text, 1) = format!("{}", settings.merge_steps);
                    // text.sections[1].value = format!("{}", settings.merge_steps)
                }
                UiSettings::AgentRadius => {
                    *text_writer.text(text, 1) = format!("{:.1}", settings.agent_radius);
                    // text.sections[1].value = format!("{:.1}", settings.agent_radius)
                }
                UiSettings::AgentRadiusOuter => (),
                UiSettings::Cache => (),
            }
        }
    }
    if example_settings.is_changed() || settings.is_changed() {
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
                UiSettings::AgentRadiusOuter => {
                    *color = if settings.agent_radius_on_outer_edge {
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
        (
            &Interaction,
            &UiSettingsButtons,
            Option<&mut BackgroundColor>,
        ),
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
                        settings.agent_radius = (settings.agent_radius - 0.1).max(0.0);
                    }
                    UiSettingsButtons::AgentRadiusInc => {
                        settings.agent_radius = (settings.agent_radius + 0.1).min(5.0);
                    }
                    UiSettingsButtons::ToggleCache => {
                        example_settings.cache_enabled = !example_settings.cache_enabled;
                    }
                    UiSettingsButtons::AgentRadiusOuterToggle => {
                        settings.agent_radius_on_outer_edge = !settings.agent_radius_on_outer_edge;
                    }
                }
            }
            Interaction::Hovered => {
                if !matches!(button, UiSettingsButtons::ToggleCache)
                    && !matches!(button, UiSettingsButtons::AgentRadiusOuterToggle)
                {
                    color.as_mut().unwrap().0 = palettes::tailwind::GRAY_600.into();
                }
            }
            Interaction::None => {
                if !matches!(button, UiSettingsButtons::ToggleCache)
                    && !matches!(button, UiSettingsButtons::AgentRadiusOuterToggle)
                {
                    color.as_mut().unwrap().0 = palettes::tailwind::GRAY_700.into();
                }
            }
        }
    }
}

pub fn setup_stats<const INTERACTIVE: bool>(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(300.0),
                ..default()
            },
            BorderRadius {
                top_left: Val::Px(0.),
                top_right: Val::Px(0.),
                bottom_right: Val::Px(20.0),
                bottom_left: Val::Px(0.),
            },
            BackgroundColor(palettes::tailwind::GRAY_900.with_alpha(0.8).into()),
            Ui,
        ))
        .with_children(|parent| {
            let mut text = vec![
                ("Status: ", 20.0),
                ("{}", 20.0),
                ("\nObstacles: ", 20.0),
                ("{}", 20.0),
                ("\nPolygons: ", 20.0),
                ("{}", 20.0),
                ("\nBuild Duration: ", 20.0),
                ("{}", 20.0),
                ("ms", 20.0),
            ];
            if INTERACTIVE {
                text.push(("\n\nClick to add an obstacle", 15.0));
                text.push(("\nPress spacebar to reset", 15.0));
            }
            parent.spawn((Text::default(), UiStats)).with_children(|p| {
                for (text, font_size) in text.into_iter() {
                    p.spawn((
                        TextSpan::new(text),
                        TextFont {
                            font_size,
                            ..default()
                        },
                    ));
                }
            });
        });
}

pub fn update_stats<T: Component>(
    mut text: Query<Entity, With<UiStats>>,
    obstacles: Query<&T>,
    navmesh: Query<(Ref<NavMeshStatus>, &NavMeshHandle)>,
    navmeshes: Res<Assets<NavMesh>>,
    diagnostics: Res<DiagnosticsStore>,
    mut text_writer: TextUiWriter,
) {
    let (status, handle) = navmesh.single();

    if !status.is_changed() && !status.is_added() {
        return;
    }

    let text = text.single_mut();
    *text_writer.text(text, 2) = format!("{:?}", *status);
    *text_writer.color(text, 2) = match *status {
        NavMeshStatus::Building => palettes::tailwind::AMBER_500.into(),
        NavMeshStatus::Built => palettes::tailwind::GREEN_400.into(),
        NavMeshStatus::Failed => palettes::tailwind::RED_600.into(),
        NavMeshStatus::Cancelled => palettes::tailwind::AMBER_500.into(),
        NavMeshStatus::Invalid => palettes::tailwind::RED_800.into(),
    };
    *text_writer.text(text, 4) = format!("{}", obstacles.iter().len());
    *text_writer.text(text, 6) = format!(
        "{}",
        navmeshes
            .get(handle.handle())
            .map(|nm| nm
                .get()
                .layers
                .iter()
                .map(|l| l.polygons.len())
                .sum::<usize>())
            .unwrap_or_default()
    );
    *text_writer.text(text, 8) = format!(
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
