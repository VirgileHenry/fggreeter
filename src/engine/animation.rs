pub use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

/// The animation event is an event sent by the state
/// machine that drives the fighting game animation
#[derive(bevy::prelude::Event)]
#[derive(Debug, Clone, Copy)]
pub enum AnimationEvent {
    /// The protagonist sends a hit to the antagonist
    Hit,
    /// The antagonist sends a hit to the protagonist
    Block,
    /// Both fighter backflip back into positions
    Reset,
    /// The protagonist charges up a super attack
    Windup,
    /// The protagonist deliver the super attack,
    /// this is followed by a nice impact frames with color changes
    /// that eventually leads to a black screen for the session start
    Finisher,
    /// The antagonist counters the charged attack
    Counter,
}

/// Fired by a clip when the fighters fall back out of contact
#[derive(bevy::animation::AnimationEvent, Clone)]
pub struct BackToDistance {
    pub node: AnimationNodeIndex,
}

/// The GLTF file with both fighters models and their animations.
/// This is a handle while the file is loading.
#[derive(Resource)]
struct FightersGltf(Handle<Gltf>);

/// Controller of the animations of the fight
#[derive(Resource)]
struct FightAnimator {
    graph: Handle<AnimationGraph>,
    fighting_state: FightingState,
    collection: FightAnimationCollection,
}

enum FightingState {
    Distant,
    Engaged,
}

/// Storage for all loaded animation clips of the fight
struct FightAnimationCollection {
    idle: AnimationNodeIndex,
    hits: Vec<AnimationNodeIndex>,
}

/// Path of the fighters model asset
const FIGHTERS_ASSET_PATH: &str = "models/fighters.glb";

/// Duration of an animation transition
const ANIM_TRANSITION_DURATION: std::time::Duration = std::time::Duration::from_millis(200);

/// Time in the hit animation it takes for the characters to close the distance
const HIT_ANIM_CLOSEUP_DURATION: std::time::Duration = std::time::Duration::from_millis(250);
/// Time in the hit animation it takes for the characters to fully perform the hit the distance
const HIT_ANIM_HIT_DURATION: std::time::Duration = std::time::Duration::from_millis(250);

pub fn add_plugins(app: &mut App) {
    app.add_systems(Startup, setup);
    app.add_systems(Update, wait_for_assets);
    app.add_observer(handle_animation_event);
    app.add_observer(on_back_to_distance);
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    /* Start loading the GLTF file with the fighters */
    let fighters_handle: Handle<Gltf> = asset_server.load(FIGHTERS_ASSET_PATH);
    let fighters_gltf = FightersGltf(fighters_handle);
    commands.insert_resource(fighters_gltf);
}

/// System that will run every frame when there is a fighter gltf file getting loaded.
/// Once we built all we need from the asset, we remove the resource and the system will never run again.
fn wait_for_assets(
    fighters_gltf: If<Res<FightersGltf>>,
    gltfs: Res<Assets<Gltf>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut commands: Commands,
) {
    /* Check whether the asset is loaded yet */
    let handle: &Handle<Gltf> = &fighters_gltf.0.0;
    let Some(gltf) = gltfs.get(handle) else { return };

    /* Create the animator */
    match create_animator(&gltf, &mut graphs, &mut clips) {
        Ok(animator) => {
            spawn_fighters(&gltf, &animator, &mut commands);
            commands.insert_resource(animator);
        }
        Err(e) => tracing::warn!("Unable to spawn fighers, could not create the animator: {e}"),
    };

    /* Asset loaded, we can throw the loading handle */
    commands.remove_resource::<FightersGltf>();
}

/// Create the animator controller for the fight from the gltf file
fn create_animator(
    fighters_gltf: &Gltf,
    graphs: &mut Assets<AnimationGraph>,
    clips: &mut Assets<AnimationClip>,
) -> Result<FightAnimator, String> {
    /* Build the animation graph */
    let mut graph = AnimationGraph::new();

    for (name, _) in fighters_gltf.named_animations.iter() {
        tracing::debug!("Found animation clip: {name}");
    }

    const IDLE_ANIM_NAME: &'static str = "Idle";
    const HIT_ANIM_PREFIX: &'static str = "Hit.";

    let Some(idle_clip) = fighters_gltf.named_animations.get(IDLE_ANIM_NAME) else {
        tracing::warn!("Failed to find an \"{IDLE_ANIM_NAME}\" animation, unable to create the animator");
        return Err(format!("Failed to find an \"{IDLE_ANIM_NAME}\""));
    };
    let hit_clips = fighters_gltf
        .named_animations
        .iter()
        .filter_map(|(name, clip)| match name.starts_with(HIT_ANIM_PREFIX) {
            true => Some(clip),
            false => None,
        })
        .collect::<Vec<_>>();
    tracing::info!("Loaded {} animation hit clips", hit_clips.len());

    let idle = graph.add_clip(idle_clip.clone(), 1.0, graph.root);
    let hits = hit_clips
        .into_iter()
        .map(|clip_handle| {
            let node = graph.add_clip(clip_handle.clone(), 1.0, graph.root);
            /* Add an event enimted by the clip to tell when we can change fighting state */
            if let Some(mut clip) = clips.get_mut(clip_handle) {
                let time = (HIT_ANIM_CLOSEUP_DURATION + HIT_ANIM_HIT_DURATION).as_secs_f32();
                clip.add_event(time, BackToDistance { node });
            }
            node
        })
        .collect::<Vec<_>>();

    let animator = FightAnimator {
        graph: graphs.add(graph),
        fighting_state: FightingState::Distant,
        collection: FightAnimationCollection { idle, hits },
    };

    Ok(animator)
}

/// Spawn the two fighters from the glb model
fn spawn_fighters(fighters_gltf: &Gltf, animator: &FightAnimator, commands: &mut Commands) {
    let fighters_prefab = fighters_gltf.scenes[0].clone();
    let world_asset = WorldAssetRoot(fighters_prefab);

    let graph = animator.graph.clone();
    let idle = animator.collection.idle;

    /* Spawn the fighters, and whenever they are spawned start the animation */
    commands.spawn(world_asset).observe(
        move |_: On<WorldInstanceReady>, mut animation_player: Single<(Entity, &mut AnimationPlayer)>, mut commands: Commands| {
            let (entity, player) = &mut *animation_player;
            let mut transitions = AnimationTransitions::new();
            transitions.play(player, idle, std::time::Duration::ZERO).repeat();

            let new_components = (AnimationGraphHandle(graph.clone()), transitions);
            commands.entity(*entity).insert(new_components);
        },
    );
}

/// Observer to update the fight animation based on the emitted animation events
fn handle_animation_event(
    event: On<AnimationEvent>,
    mut animator: ResMut<FightAnimator>,
    mut fight: Single<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let (animation_player, animation_transition) = &mut *fight;
    match *event {
        AnimationEvent::Hit => hit(&mut *animator, animation_player, animation_transition),
        other => tracing::warn!("Unhandled animation event: {other:?}"),
    }
}

/// Hit animation event, the protagonsit hits the antagonist
fn hit(animator: &mut FightAnimator, animation_player: &mut AnimationPlayer, animation_transition: &mut AnimationTransitions) {
    /* Pick a random hit to play */
    use rand::seq::IndexedRandom;
    let mut rng = rand::rng();
    let Some(hit_clip) = animator.collection.hits.choose(&mut rng) else {
        tracing::warn!("No hit animations clip: unable to play hit animation");
        return;
    };

    /* If the characters are already close to each other, start the anim at the hit */
    let start_duration = match animator.fighting_state {
        FightingState::Distant => std::time::Duration::ZERO,
        FightingState::Engaged => HIT_ANIM_CLOSEUP_DURATION,
    };

    /* Play the hit */
    let active_anim = animation_transition.play(animation_player, *hit_clip, ANIM_TRANSITION_DURATION);
    active_anim.seek_to(start_duration.as_secs_f32());

    /* Engaged from the moment a hit starts, so rapid typing always chains */
    animator.fighting_state = FightingState::Engaged;
}

/// Observer to handle animation events to get the state back to distant
fn on_back_to_distance(event: On<BackToDistance>, rig: Single<&AnimationTransitions>, mut animator: ResMut<FightAnimator>) {
    if rig.get_main_animation() == Some(event.node) {
        animator.fighting_state = FightingState::Distant;
    }
}
