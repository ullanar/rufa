//! Effect ids the graphs classify items by, resolved once from the catalog.

use std::collections::HashMap;

use rufa_data::{Catalog, Info};

macro_rules! effect_ids {
    ($($field:ident = $name:literal,)*) => {
        #[derive(Debug, Default)]
        pub struct EffectIds {
            $(pub $field: Option<i32>,)*
            cache: std::sync::RwLock<HashMap<i32, Vec<i32>>>,
        }

        impl EffectIds {
            pub fn resolve(catalog: &Catalog) -> EffectIds {
                EffectIds {
                    $($field: catalog.effect_id($name),)*
                    cache: Default::default(),
                }
            }
        }
    };
}

effect_ids! {
    turret_fitted = "turretFitted",
    launcher_fitted = "launcherFitted",
    use_missiles = "useMissiles",
    target_attack = "targetAttack",
    projectile_fired = "projectileFired",
    mining_laser = "miningLaser",
    fof_missile = "fofMissileLaunching",
    dot_missile = "dotMissileLaunching",
    chain_lightning = "ChainLightning",
    emp_wave = "empWave",
    super_weapon_amarr = "superWeaponAmarr",
    super_weapon_caldari = "superWeaponCaldari",
    super_weapon_gallente = "superWeaponGallente",
    super_weapon_minmatar = "superWeaponMinmatar",
    lightning_weapon = "lightningWeapon",
    doomsday_beam_dot = "doomsdayBeamDOT",
    doomsday_slash = "doomsdaySlash",
    doomsday_cone_dot = "doomsdayConeDOT",
    debuff_lance = "debuffLance",
    web = "remoteWebifierFalloff",
    structure_web = "structureModuleEffectStasisWebifier",
    burst_web = "doomsdayAOEWeb",
    web_drone = "remoteWebifierEntity",
    web_fighter = "fighterAbilityStasisWebifier",
    paint = "remoteTargetPaintFalloff",
    structure_paint = "structureModuleEffectTargetPainter",
    burst_paint = "doomsdayAOEPaint",
    paint_drone = "remoteTargetPaintEntity",
    scram = "warpScrambleBlockMWDWithNPCEffect",
    structure_scram = "structureWarpScrambleBlockMWDWithNPCEffect",
    bubble = "warpDisruptSphere",
    mwd = "moduleBonusMicrowarpdrive",
    afterburner = "moduleBonusAfterburner",
    micro_jump = "microJumpDrive",
    micro_jump_portal = "microJumpPortalDrive",
    neut = "energyNeutralizerFalloff",
    structure_neut = "structureEnergyNeutralizerFalloff",
    nos = "energyNosferatuFalloff",
    burst_neut = "doomsdayAOENeut",
    neut_drone = "entityEnergyNeutralizerFalloff",
    neut_fighter = "fighterAbilityEnergyNeutralizer",
    ecm = "remoteECMFalloff",
    structure_ecm = "structureModuleEffectECM",
    burst_ecm = "doomsdayAOEECM",
    ecm_drone = "entityECMFalloff",
    ecm_fighter = "fighterAbilityECM",
    damp = "remoteSensorDampFalloff",
    structure_damp = "structureModuleEffectRemoteSensorDampener",
    burst_damp = "doomsdayAOEDamp",
    damp_drone = "remoteSensorDampEntity",
    tracking_disruptor = "shipModuleTrackingDisruptor",
    structure_weapon_disruptor = "structureModuleEffectWeaponDisruption",
    burst_track = "doomsdayAOETrack",
    weapon_disruptor_drone = "npcEntityWeaponDisruptor",
    guidance_disruptor = "shipModuleGuidanceDisruptor",
    ancillary_remote_armor = "shipModuleAncillaryRemoteArmorRepairer",
    ancillary_remote_shield = "shipModuleAncillaryRemoteShieldBooster",
    fighter_attack_missile = "fighterAbilityAttackM",
    fighter_missiles = "fighterAbilityMissiles",
    fighter_bomb = "fighterAbilityLaunchBomb",
}

impl EffectIds {
    /// The effect ids of a type, kept after the first lookup.
    pub fn of(&self, info: &impl Info, type_id: i32) -> Vec<i32> {
        if let Some(list) = self
            .cache
            .read()
            .ok()
            .and_then(|c| c.get(&type_id).cloned())
        {
            return list;
        }
        let list: Vec<i32> = info
            .get_dogma_effects(type_id)
            .into_iter()
            .flatten()
            .map(|e| e.effect_id())
            .collect();
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(type_id, list.clone());
        }
        list
    }

    pub fn has(&self, info: &impl Info, type_id: i32, effect: Option<i32>) -> bool {
        match effect {
            Some(id) => self.of(info, type_id).contains(&id),
            None => false,
        }
    }

    pub fn has_any(&self, info: &impl Info, type_id: i32, effects: &[Option<i32>]) -> bool {
        let list = self.of(info, type_id);
        effects.iter().flatten().any(|id| list.contains(id))
    }
}
