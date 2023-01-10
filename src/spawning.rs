use bevy::ecs::system::EntityCommands;
use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy_rapier3d::parry::transformation::utils::transform;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

mod doorway;
mod grass;
mod wall;
use crate::GameState;
use strum_macros::EnumIter;

pub struct GameObjectsPlugin;

impl Plugin for GameObjectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEvent>()
            .init_resource::<SpawnContainerRegistry>()
            .add_startup_system(load_assets_for_spawner)
            .add_system_set(SystemSet::on_update(GameState::Playing).with_system(spawn_requested));
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnEvent {
    pub object: GameObject,
    pub transform: Transform,
    pub parent: Option<Cow<'static, str>>,
}

pub struct SpawnEventSender {
    pub object: GameObject,
    pub transform: Transform,
    pub parent: Option<Cow<'static, str>>,
}

impl SpawnEventSender {
    pub fn new(object: GameObject) -> Self {
        Self {
            object,
            transform: default(),
            parent: default(),
        }
    }

    pub fn with_transform(&mut self, transform: Transform) -> &mut Self {
        self.transform = transform;
        self
    }

    pub fn with_translation(&mut self, translation: impl Into<Vec3>) -> &mut Self {
        self.transform = Transform::from_translation(translation.into());
        self
    }

    pub fn with_parent(&mut self, parent: impl Into<Cow<'static, str>>) -> &mut Self {
        self.parent = Some(parent.into());
        self
    }

    pub fn send(&mut self, events: &mut EventWriter<SpawnEvent>) -> &mut Self {
        let event = SpawnEvent {
            object: self.object,
            transform: self.transform,
            parent: self.parent.clone(),
        };
        events.send(event);
        self
    }
}

#[derive(Debug, EnumIter, Clone, Copy, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum GameObject {
    Grass,
    Doorway,
    Wall,
}

#[derive(Resource)]
pub struct GameObjectSpawner {
    meshes: HashMap<GameObject, Handle<Mesh>>,
    materials: HashMap<GameObject, Handle<StandardMaterial>>,
    scenes: HashMap<GameObject, Handle<Gltf>>,
}

#[derive(Resource)]
pub struct PrimedGameObjectSpawner<'w, 's, 'a, 'b> {
    handles: &'a GameObjectSpawner,
    gltf: &'a Res<'a, Assets<Gltf>>,
    commands: &'a mut ChildBuilder<'w, 's, 'b>,
}

impl<'a, 'b, 'c, 'w, 's> GameObjectSpawner
where
    'c: 'a,
{
    pub fn attach(
        &'c self,
        commands: &'a mut ChildBuilder<'w, 's, 'b>,
        gltf: &'a Res<'a, Assets<Gltf>>,
    ) -> PrimedGameObjectSpawner<'w, 's, 'a, 'b> {
        PrimedGameObjectSpawner {
            handles: self,
            commands,
            gltf,
        }
    }
}

impl<'w, 's, 'a, 'b> PrimedGameObjectSpawner<'w, 's, 'a, 'b> {
    pub fn spawn(&'a mut self, object: &GameObject) -> EntityCommands<'w, 's, 'a> {
        match *object {
            GameObject::Grass => self.spawn_grass(),
            GameObject::Doorway => self.spawn_doorway(),
            GameObject::Wall => self.spawn_wall(),
        }
    }
}

fn load_assets_for_spawner(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    let mut meshes = HashMap::new();
    meshes.insert(GameObject::Grass, grass::create_mesh(&mut mesh_assets));

    let mut materials = HashMap::new();
    materials.insert(
        GameObject::Grass,
        grass::load_material(&asset_server, &mut material_assets),
    );

    let mut scenes = HashMap::new();
    scenes.insert(GameObject::Doorway, doorway::load_scene(&asset_server));
    scenes.insert(GameObject::Wall, wall::load_scene(&asset_server));

    commands.insert_resource(GameObjectSpawner {
        meshes,
        materials,
        scenes,
    });
}

#[derive(Debug, Clone, Eq, PartialEq, Resource, Reflect, Serialize, Deserialize, Default)]
#[reflect(Resource, Serialize, Deserialize)]
struct SpawnContainerRegistry(HashMap<Cow<'static, str>, Entity>);

fn spawn_requested(
    mut commands: Commands,
    gltf: Res<Assets<Gltf>>,
    mut spawn_requests: EventReader<SpawnEvent>,
    spawner: Res<GameObjectSpawner>,
    mut spawn_containers: ResMut<SpawnContainerRegistry>,
) {
    for spawn in spawn_requests.iter() {
        let bundle = (
            Name::new(format!("{:?}", spawn.object)),
            VisibilityBundle::default(),
            TransformBundle::from_transform(spawn.transform),
        );
        let spawn_children = |parent: &mut ChildBuilder| {
            spawner.attach(parent, &gltf).spawn(&spawn.object);
        };

        if let Some(ref parent_name) = spawn.parent {
            // command.spawn() takes a tick to actually spawn stuff,
            // so we need to keep an own list of already "spawned" parents
            let parent = spawn_containers
                .0
                .entry(parent_name.to_owned())
                .or_insert_with(|| {
                    commands
                        .spawn((
                            Name::new(parent_name.clone()),
                            VisibilityBundle::default(),
                            TransformBundle::default(),
                        ))
                        .id()
                });

            commands.entity(*parent).with_children(|parent| {
                parent.spawn(bundle).with_children(spawn_children);
            });
        } else {
            commands.spawn(bundle).with_children(spawn_children);
        }
    }
}
