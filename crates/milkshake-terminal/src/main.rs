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
        .spawn((
            NodeBundle {
                background_color: Color::BLACK.with_alpha(0.1).into(),
                style: Style {
                    aspect_ratio: Some(1.0),
                    display: Display::Grid,
                    width: Val::Px(1000.0),
                    height: Val::Px(1000.0),
                    grid_template_columns: RepeatedGridTrack::flex(105, 1.0),
                    grid_template_rows: RepeatedGridTrack::flex(65, 1.0),
                    ..default()
                },
                ..default()
            },
            Terminal {
                program: "fish".into(),
                text_style: style.clone(),
            },
        ))
        .with_children(|_builder| {
            // Intentionally empty, just to insert the Children component.
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
