use bevy_asset::{Asset, AssetId, AssetPath, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query, ResMut},
};
use bevy_hierarchy::{Children, HierarchyQueryExt, Parent};
use bevy_math::Vec3;
use bevy_reflect::{Reflect, TypePath};
use bevy_transform::components::Transform;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::AnimationGraph, AnimationClip, AnimationPlayer, AnimationTarget, Interpolation, Keyframes, VariableCurve
};

#[derive(Debug, Reflect, Clone, Serialize, Deserialize)]
pub enum RootMotionBakeType {
    Cog,
    RootBone,
}

#[derive(Asset, TypePath, Debug)]
pub struct RootMotionCurve(pub VariableCurve);

#[derive(Debug, Reflect, Clone)]
pub struct RootMotionData {
    pub bake_type: RootMotionBakeType,
    pub curve: Option<Handle<RootMotionCurve>>,
}

/// A version of `RootMotionData` suitable for serializing as an asset.
#[derive(Serialize, Deserialize)]
pub struct SerializedRootMotionData {
    pub bake_type: RootMotionBakeType,
    pub curve: SerializedRootMotionCurve,
}

/// A version of `RootMotionCurve` suitable for serializing as an asset.
///
/// This replaces any handle that has a path with an [`AssetPath`]. Failing
/// that, the asset ID is serialized directly.
#[derive(Serialize, Deserialize)]
pub enum SerializedRootMotionCurve {
    /// Records an asset path.
    AssetPath(AssetPath<'static>),
    /// The fallback that records an asset ID.
    ///
    /// Because asset IDs can change, this should not be relied upon. Prefer to
    /// use asset paths where possible.
    AssetId(AssetId<RootMotionCurve>),
}

#[allow(clippy::too_many_arguments)]
pub fn bake_root_motion_system(
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut root_motion_curves: ResMut<Assets<RootMotionCurve>>,
    mut q_graph_owners: Query<(Entity, &mut AnimationPlayer, &Handle<AnimationGraph>)>,
    mut q_transforms: Query<&mut Transform>,
    q_targets: Query<(Entity, &AnimationTarget, Option<&Name>)>,
    q_names: Query<&Name>,
    q_parents: Query<&Parent>,
    q_children: Query<&Children>,
    mut commands: Commands,
) {
    let sample_rate = 60.0;
    let step_size = 1.0 / sample_rate;
    let mut all_baked = true;

    for (root_entity, player, graph_handle) in q_graph_owners.iter_mut() {
        let Some(graph) = graphs.get_mut(graph_handle) else {
            continue;
        };
        for node_id in graph.nodes() {
            let Some(node) = graph.get_mut(node_id) else {
                continue;
            };
            let Some(root_motion) = node.root_motion.clone() else {
                continue;
            };
            // match root_motion.bake_type {
            //     RootMotionBakeType::Cog => todo!(),
            //     RootMotionBakeType::RootBone => todo!(),
            // }

            let Some(clip) = node.clip.as_ref().and_then(|handle| clips.get(handle)) else {
                continue;
            };

            let initial_cog = get_cog(
                root_entity,
                &q_children,
                &q_transforms,
                &q_parents,
                &q_names,
            );

            let mut timestamps: Vec<f32> = Vec::new();
            let mut keyframes: Vec<Vec3> = Vec::new();

            let mut elapsed = 0.0;
            while elapsed <= clip.duration() {
                player.play(node_id).seek_to(elapsed);
                animate_targets_imperative(clip, &animation_player, &q_targets, &mut q_transforms);
                let cog = get_cog(
                    root_entity,
                    &q_children,
                    &q_transforms,
                    &q_parents,
                    &q_names,
                );
                timestamps.push(elapsed);
                keyframes.push(cog);
                elapsed += step_size;
            }
            assert_eq!(timestamps.len(), keyframes.len());
            let root_motion_curve = RootMotionCurve(VariableCurve {
                keyframe_timestamps: timestamps,
                keyframes: Keyframes::Translation(keyframes),
                interpolation: Interpolation::Linear,
            });
        }
    }
}

//     for (name, data) in animations.animations.iter_mut() {
//         if data.bake_type == AnimationBakeType::None || data.root_motion.is_some() {
//             continue;
//         } else {
//             all_baked = false;
//         }
//         let Some(avatar_entity) = data.avatar else {
//             continue;
//         };
//         let Ok(rig_root) = q_animation_owners.get(avatar_entity).map(|it| it.0) else {
//             continue;
//         };
//         let Some(clip) = clips.get_mut() else {
//             continue;
//         };
//         // Map animation target IDs to entities.
//         let target_id_to_entity_map: HashMap<_, _> = q_targets
//             .iter()
//             .filter(|(_e, target, _n)| target.player == rig_root)
//             .map(|(entity, target, _n)| (target.id, entity))
//             .collect();
//
//         let Ok((mut animation_player, graph_handle)) = q_graph_owners.get_mut(rig_root) else {
//             continue;
//         };
//
//         let Some(graph) = graphs.get_mut(graph_handle) else {
//             continue;
//         };
//         let Some(clip) = graph
//             .get(data.clip)
//             .and_then(|animation_graph_node| animation_graph_node.clip.as_ref())
//             .and_then(|animation_clip_handle| clips.get(animation_clip_handle))
//         else {
//             warn!("Couldnt load: {name}");
//             continue;
//         };
//
//         let initial_cog = get_cog(rig_root, &q_children, &q_transforms, &q_parents, &q_names);
//
//         let mut timestamps: Vec<f32> = Vec::new();
//         let mut keyframes: Vec<Vec3> = Vec::new();
//
//         let mut elapsed = 0.0;
//         while elapsed <= clip.duration() {
//             animation_player.play(data.clip).seek_to(elapsed);
//             animate_targets_imperative(clip, &animation_player, &q_targets, &mut q_transforms);
//             let cog = get_cog(rig_root, &q_children, &q_transforms, &q_parents, &q_names);
//             timestamps.push(elapsed);
//             keyframes.push(cog);
//             elapsed += step_size;
//         }
//         assert_eq!(timestamps.len(), keyframes.len());
//         data.root_motion = Some(VariableCurve {
//             keyframe_timestamps: timestamps,
//             keyframes: Keyframes::Translation(keyframes),
//             interpolation: Interpolation::Linear,
//         });
//         let baked_clip = bake_root_motion_clip(
//             clip,
//             initial_cog,
//             data.root_motion.as_ref().unwrap(),
//             data.bake_type,
//             target_id_to_entity_map.clone(),
//             rig_root,
//             &q_parents,
//         );
//         let clip_handle = clips.add(baked_clip);
//         let node_index = graph.add_clip(clip_handle, 1.0, graph.root);
//         data.clip_baked = Some(node_index);
//         info!("Baked root motion curve for: {name}");
//     }
//     if all_baked {
//         commands.remove_resource::<RootMotionNeedsBaking>();
//     }
// }

fn get_cog(
    root: Entity,
    q_children: &Query<&Children>,
    q_transforms: &Query<&mut Transform>,
    q_parents: &Query<&Parent>,
    q_names: &Query<&Name>,
) -> Vec3 {
    let mut sum = Vec3::ZERO;
    let mut count = 0;
    for child in q_children.iter_descendants(root) {
        if let Ok(name) = q_names.get(child) {
            if !name.starts_with("DEF") {
                continue;
            }
        }
        count += 1;
        let mut current_entity = child;
        let mut res_transform = Transform::IDENTITY;
        loop {
            let Ok(transform) = q_transforms.get(current_entity) else {
                break;
            };
            let Ok(parent) = q_parents.get(current_entity) else {
                break;
            };
            res_transform = (*transform).mul_transform(res_transform);
            if parent.get() == root {
                break;
            }
            current_entity = parent.get();
        }
        sum += res_transform.translation;
    }
    sum / (count as f32)
}
