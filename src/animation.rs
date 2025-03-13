pub struct AnimationPlugin;

use std::{collections::HashSet, f32::consts::PI, time::Duration};

use avian3d::prelude::LinearVelocity;
use bevy::{
    animation::{AnimationTarget, AnimationTargetId},
    prelude::*,
    render::{mesh::skinning::SkinnedMesh, view::NoFrustumCulling},
};

use crate::{camera::PlayerMarker, consts::CHARACTER_MODEL_PATH, input::MovementIntent};

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, setup_scene_once_loaded)
            .add_systems(Update, disable_culling_for_skinned_meshes)
            .add_systems(Update, get_neck_bone)
            .add_systems(Update, keyboard_input_test)
            .add_systems(Update, handle_run_animation)
            .add_systems(Update, link_animations);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Name::new("running dude"),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(CHARACTER_MODEL_PATH))),
    ));
}

#[derive(Clone, Debug, Resource)]
struct PlayerAnimationNodes([AnimationNodeIndex; 2]);

// indexes for animation node index above
const PLAYER_IDLE: usize = 0;
const PLAYER_RUN: usize = 1;

fn setup_scene_once_loaded(
    mut commands: Commands,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    targets: Query<(Entity, &AnimationTarget)>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut player) in &mut players {
        debug!("got an animation player");
        // Load the animation clip from the glTF file.

        let mut animation_graph = AnimationGraph::new();
        let blend_node = animation_graph.add_additive_blend(1.0, animation_graph.root);
        let mut animation_graph_nodes: [AnimationNodeIndex; 2] = [Default::default(); 2];
        /*

           std::array::from_fn(|animation_index| {
               let handle = asset_server.load(
                   GltfAssetLabel::Animation(animation_index).from_asset("models/character.glb"),
               );
               let mask = if animation_index == 0 { 0 } else { 0 };

               animation_graph.add_clip_with_mask(handle, mask, 1.0, blend_node)
           });
        */
        let mut handle =
            asset_server.load(GltfAssetLabel::Animation(2).from_asset(CHARACTER_MODEL_PATH));

        let mask = 1;
        animation_graph_nodes[0] =
            animation_graph.add_clip_with_mask(handle, mask, 1.0, blend_node);

        handle = asset_server.load(GltfAssetLabel::Animation(3).from_asset(CHARACTER_MODEL_PATH));

        animation_graph_nodes[1] =
            animation_graph.add_clip_with_mask(handle, mask, 1.0, blend_node);

        commands.insert_resource(PlayerAnimationNodes(animation_graph_nodes));

        // Create each mask group.
        let mut all_animation_target_ids = HashSet::new();

        for (mask_group_index, (mask_group_prefix, mask_group_suffix)) in
            MASK_GROUP_PATHS.iter().enumerate()
        {
            // Split up the prefix and suffix, and convert them into `Name`s.
            let prefix: Vec<_> = mask_group_prefix.split('/').map(Name::new).collect();
            let suffix: Vec<_> = mask_group_suffix.split('/').map(Name::new).collect();
            // Add each bone in the chain to the appropriate mask group.

            for chain_length in 0..=suffix.len() {
                let animation_target_id = AnimationTargetId::from_names(
                    prefix.iter().chain(suffix[0..chain_length].iter()),
                );

                debug!("add_target_to_mask_group");
                animation_graph
                    .add_target_to_mask_group(animation_target_id, mask_group_index as u32);

                all_animation_target_ids.insert(animation_target_id);
            }
        }
        debug!("mask groups {:?}", animation_graph.mask_groups);
        // We're done constructing the animation graph. Add it as an asset.

        let animation_graph = animation_graphs.add(animation_graph);

        //let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        /*
        transitions
            .play(&mut player, animations.animations[1], Duration::ZERO)
            .repeat();


         */
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animation_graph))
            .insert(Name::new("Animation player"));

        // Remove animation targets that aren't in any of the mask groups. If we
        // don't do that, those bones will play all animations at once, which is
        // ugly.

        /*
        for (target_entity, target) in &targets {
            if !all_animation_target_ids.contains(&target.id) {
                commands.entity(target_entity).remove::<AnimationTarget>();
            }
        }

        */

        // Play the animation.

        // play running animation
        player.play(animation_graph_nodes[1]).repeat();

        // Record the graph nodes.
        //commands.insert_resource(AnimationNodes(animation_graph_nodes));
    }
}

/// https://github.com/bevyengine/bevy/issues/4971
fn disable_culling_for_skinned_meshes(
    mut commands: Commands,
    skinned: Query<Entity, Added<SkinnedMesh>>,
) {
    for entity in &skinned {
        debug!("adding no frustum");
        commands.entity(entity).insert(NoFrustumCulling);
    }
}

/*

#[derive(Clone, Debug, Resource)]

struct AnimationNodes([AnimationNodeIndex; 2]);
*/

#[derive(Component)]
struct HeadJoint(Entity); // holds entity to the internal bevy joint. used for rotating head when looking around

// there's a fix to avoid this in 0.15 but looks too complicated
// https://www.reddit.com/r/bevy/comments/1h5l1oj/is_there_any_way_to_create_a_skinned_mesh_from_a/
fn get_neck_bone(
    mut commands: Commands,
    skinned: Query<(&Name, Entity, &SkinnedMesh), Added<SkinnedMesh>>,
    bones_q: Query<(&Name, Entity)>,
) {
    for (name, entity, skined_mesh) in &skinned {
        debug!("Name of mesh: {:?}", name);
        if **name == *"Cube.001" {
            for joint in skined_mesh.joints.iter() {
                if let Ok((bone_name, bone_ent)) = bones_q.get(*joint) {
                    debug!("bone_name {:?}", bone_name);
                    if **bone_name == *"spine.005" {
                        debug!("FOUND MAGIC SPINE BONE");
                        commands.spawn((HeadJoint(bone_ent), Name::new("HEAD JOINT")));
                    }
                }
            }
        }
    }
}

fn keyboard_input_test(
    keys: Res<ButtonInput<KeyCode>>,
    head_joint: Query<&HeadJoint>,
    mut bones_q: Query<(&mut Transform)>,
) {
    if keys.pressed(KeyCode::KeyL) {
        debug!("pressed L");
        for joint in head_joint.iter() {
            debug!("head joint exists");
            if let Ok(mut bone_tf) = bones_q.get_mut(joint.0) {
                debug!("accessed bone_tf {:?}", bone_tf);

                bone_tf.rotate_local_y(PI / 10.);
            }
        }
    }
}

// need to mask the head/body so we can
const MASK_GROUP_PATHS: [(&str, &str); 1] = [
    // Head
    (
        "metarig/spine/spine.001/spine.002/spine.003",
        "spine.004/spine.005/spine.006/face",
    ),
    // test
    //("metarig", "spine/thigh.L/shin.L/foot.L"),
];

//https://github.com/bevyengine/bevy/discussions/5564

#[derive(Component)]
pub struct AnimationEntityLink(pub Entity);

fn get_top_parent(mut curr_entity: Entity, parent_query: &Query<&Parent>) -> Entity {
    //Loop up all the way to the top parent
    loop {
        if let Ok(parent) = parent_query.get(curr_entity) {
            curr_entity = parent.get();
        } else {
            break;
        }
    }
    curr_entity
}
pub fn link_animations(
    player_query: Query<Entity, Added<AnimationPlayer>>,
    parent_query: Query<&Parent>,
    animations_entity_link_query: Query<&AnimationEntityLink>,
    mut commands: Commands,
) {
    // Get all the Animation players which can be deep and hidden in the heirachy
    for entity in player_query.iter() {
        let top_entity = get_top_parent(entity, &parent_query);

        // If the top parent has an animation config ref then link the player to the config
        if animations_entity_link_query.get(top_entity).is_ok() {
            warn!("Problem with multiple animationsplayers for the same top parent");
        } else {
            commands
                .entity(top_entity)
                .insert(AnimationEntityLink(entity.clone()));
        }
    }
}

fn handle_run_animation(
    mut anim_player_q: Query<&mut AnimationPlayer>,
    player_q: Query<(&LinearVelocity, &AnimationEntityLink), With<PlayerMarker>>,
    anim_nodes: Option<Res<PlayerAnimationNodes>>,
) {
    let Some(anim_nodes) = anim_nodes else {
        debug!("unable to find anim_player");
        return;
    };
    for (vel, animation_link) in player_q.iter() {
        let Ok(mut anim_player) = anim_player_q.get_mut(animation_link.0) else {
            debug!("unable to find anim_player");
            continue;
        };

        // todo should check MoveIntent instead of velocity, but MoveIntent is not shared over the network atm
        if vel.0.length() < 0.5 && anim_player.is_playing_animation(anim_nodes.0[PLAYER_RUN]) {
            debug!("Setting player anim to PLAYER_IDLE");
            anim_player.stop_all();
            anim_player.play(anim_nodes.0[PLAYER_IDLE]).repeat();
        }
        if vel.0.length() >= 0.5 && anim_player.is_playing_animation(anim_nodes.0[PLAYER_IDLE]) {
            debug!("Setting player anim to PLAYER_RUN");
            anim_player.stop_all();
            anim_player.play(anim_nodes.0[PLAYER_RUN]).repeat();
        }
    }
}
