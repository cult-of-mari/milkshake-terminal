use self::terminal::Terminal;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_resource::{
    Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::view::RenderLayers;
use leafwing_input_manager::prelude::*;

mod terminal;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Reflect)]
pub enum Action {
    Jump,
    Look,
    Move,
}

impl Actionlike for Action {
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            Self::Jump => InputControlKind::Button,
            Self::Look => InputControlKind::DualAxis,
            Self::Move => InputControlKind::DualAxis,
        }
    }
}

fn setup(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let roboto_mono = asset_server.load("fonts/RobotoMono-SemiBold.ttf");
    let chimera_logo = asset_server.load("images/Chimera-Logo.png");

    let style = TextStyle {
        font: roboto_mono,
        font_size: 20.0,
        color: Color::BLACK,
    };

    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    image.resize(size);

    let image_handle = images.add(image);

    commands
        .spawn(SpatialBundle::default())
        .with_children(|parent| {
            parent.spawn((
                Camera3dBundle {
                    camera: Camera {
                        clear_color: ClearColorConfig::Custom(Color::NONE),
                        order: -1,
                        target: RenderTarget::Image(image_handle.clone()),
                        ..default()
                    },
                    ..default()
                },
                RenderLayers::layer(1),
            ));

            parent.spawn((
                PbrBundle {
                    mesh: meshes.add(Cuboid::default()),
                    material: materials.add(StandardMaterial {
                        base_color: Color::WHITE,
                        base_color_texture: Some(chimera_logo.clone()),
                        emissive: Color::WHITE.into(),
                        emissive_texture: Some(chimera_logo.clone()),
                        ..default()
                    }),
                    transform: Transform::from_xyz(0.0, 0.0, -50.0).with_scale(Vec3::splat(20.0)),
                    ..default()
                },
                RigidBody::Kinematic,
                AngularVelocity(Vec3::new(1.0, 1.0, 1.0)),
                RenderLayers::layer(1),
            ));
        });

    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                margin: UiRect::all(Val::Px(24.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle {
                text: Text::from_sections([TextSection::new("\n~ 1000\n> \n", style.clone())]),
                ..default()
            });

            parent.spawn(NodeBundle::default()).with_children(|parent| {
                parent.spawn(ImageBundle {
                    style: Style {
                        height: Val::Px(256.0),
                        width: Val::Px(256.0),
                        ..default()
                    },
                    image: image_handle.into(),
                    ..default()
                });

                let lines = [
                    "mari@puter",
                    "----------",
                    "OS: Chimera Linux x86_64",
                    "Host: MS-7B79 (3.0)",
                    "Kernel: Linux 6.11.4-0-generic",
                    "Uptime: 57 mins",
                    "Packages: 1346 (apk), 8 (flatpak-user)",
                    "Shell: fish 3.7.1",
                    "Display (C32JG5x): 2560x1440 @ 144 Hz in 32\" []",
                    "WM: Sway (Wayland)",
                    "Terminal: (this part isnt real yet)",
                    "Terminal Font: Roboto Mono SemiBold [GOOG] (11)",
                    "CPU: AMD Ryzen 9 3900X (24) @ 4.67 GHz",
                    "GPU: NVIDIA GeForce RTX 2060",
                    "Memory: 5.50 GiB / 125.74 GiB (4%)",
                    "Swap: Disabled",
                    "Disk (/): 327.47 GiB / 3.64 TiB (9%) - xfs",
                    "Local IP (enp34s0): 192.168.20.11/24",
                    "Locale: C.UTF-8",
                ];

                let lines = lines
                    .iter()
                    .map(|line| TextSection::new(format!("{line}\n"), style.clone()))
                    .collect::<Vec<_>>();

                parent.spawn(TextBundle {
                    text: Text::from_sections(lines),
                    ..default()
                });
            });

            parent.spawn((
                TextBundle::default(),
                Terminal {
                    program: "fish".into(),
                    text_style: style.clone(),
                },
            ));
        });

    commands.spawn(Camera3dBundle::default());
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::WHITE))
        .add_plugins((
            DefaultPlugins,
            InputManagerPlugin::<Action>::default(),
            PhysicsPlugins::default(),
            terminal::TerminalPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}
