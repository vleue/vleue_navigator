use bevy::{color::palettes, prelude::*};
use vleue_navigator::{
    updater::{NavMeshSettings, NavMeshStatus},
    NavMesh,
};

#[derive(Component)]
pub enum UiSettings {
    Simplify,
    MergeSteps,
}

#[derive(Component)]
pub enum UiSettingsButtons {
    SimplifyInc,
    SimplifyDec,
    MergeStepsInc,
    MergeStepsDec,
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
                image: UiImage::default().with_color(palettes::tailwind::GRAY_700.into()),
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

pub fn setup_settings(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
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
            background_color: BackgroundColor(palettes::tailwind::GRAY_900.with_alpha(0.8).into()),
            ..default()
        })
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
        });
}

pub fn display_settings(
    settings: Query<Ref<NavMeshSettings>>,
    mut texts: Query<(&mut Text, &UiSettings)>,
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
            }
        }
    }
}

pub fn update_settings<const STEP: u32>(
    mut interaction_query: Query<
        (&Interaction, &UiSettingsButtons, &mut UiImage),
        (Changed<Interaction>, With<Button>),
    >,
    mut settings: Query<&mut NavMeshSettings>,
) {
    for (interaction, button, mut image) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                let mut settings = settings.single_mut();
                match *button {
                    UiSettingsButtons::SimplifyDec => {
                        settings.simplify = (settings.simplify - STEP as f32 / 1000.0).max(0.0);
                    }
                    UiSettingsButtons::SimplifyInc => {
                        settings.simplify = (settings.simplify + STEP as f32 / 1000.0).min(0.5);
                    }
                    UiSettingsButtons::MergeStepsDec => {
                        settings.merge_steps = settings.merge_steps.checked_sub(1).unwrap_or(0);
                    }
                    UiSettingsButtons::MergeStepsInc => {
                        settings.merge_steps = (settings.merge_steps + 1).min(5);
                    }
                }
            }
            Interaction::Hovered => {
                image.color = palettes::tailwind::GRAY_600.into();
            }
            Interaction::None => {
                image.color = palettes::tailwind::GRAY_700.into();
            }
        }
    }
}

pub fn setup_stats(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            border_radius: BorderRadius {
                top_left: Val::Px(0.),
                top_right: Val::Px(0.),
                bottom_right: Val::Px(20.0),
                bottom_left: Val::Px(0.),
            },
            background_color: BackgroundColor(palettes::tailwind::GRAY_900.with_alpha(0.8).into()),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                TextBundle {
                    text: Text::from_sections(
                        [
                            ("Status: ", 30.0),
                            ("{}", 30.0),
                            ("\nObstacles: ", 30.0),
                            ("{}", 30.0),
                            ("\nPolygons: ", 30.0),
                            ("{}", 30.0),
                            ("\n\nClick to add an obstacle", 25.0),
                            ("\nPress spacebar to reset", 25.0),
                        ]
                        .into_iter()
                        .map(|(text, font_size): (&str, f32)| {
                            TextSection::new(
                                text,
                                TextStyle {
                                    font_size,
                                    ..default()
                                },
                            )
                        }),
                    ),
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
    };
    text.sections[3].value = format!("{}", obstacles.iter().len());
    text.sections[5].value = format!(
        "{}",
        navmeshes
            .get(handle)
            .map(|nm| nm.get().polygons.len())
            .unwrap_or_default()
    );
}

#[derive(Component)]
pub struct UiStats;
