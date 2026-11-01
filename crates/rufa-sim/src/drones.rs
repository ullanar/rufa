//! Drone bay: what the hull carries and what it puts in space.
//! A drone type is up to two fit items, launched and stowed.

use rufa_data::Info;

use crate::attrs::AttrIds;
use crate::place::new_item;
use crate::{Calculation, Fit, Slot, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stack {
    pub launched: u32,
    pub stowed: u32,
}

impl Stack {
    pub fn total(self) -> u32 {
        self.launched + self.stowed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Room {
    pub bay: u32,
    pub launch: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Load {
    pub bay_used: f64,
    pub bay_total: f64,
    pub bandwidth_used: f64,
    pub bandwidth_total: f64,
    pub launched: u32,
    pub max_launched: u32,
}

impl Load {
    pub fn over(self) -> bool {
        self.launched > self.max_launched
            || self.bay_used > self.bay_total
            || self.bandwidth_used > self.bandwidth_total
    }
}

fn ship_attr(calc: &Calculation, id: i32) -> f64 {
    calc.ship.attributes.get(&id).map(|a| a.value).unwrap_or(0.0)
}

fn character_attr(calc: &Calculation, id: i32) -> f64 {
    calc.character
        .attributes
        .get(&id)
        .map(|a| a.value)
        .unwrap_or(0.0)
}

pub fn volume(info: &impl Info, type_id: i32) -> f64 {
    info.get_type(type_id)
        .and_then(|ty| ty.volume())
        .map(f64::from)
        .unwrap_or(0.0)
}

pub fn bandwidth(info: &impl Info, ids: &AttrIds, type_id: i32) -> f64 {
    info.get_dogma_attributes(type_id)
        .into_iter()
        .flatten()
        .find(|a| a.attribute_id() == ids.drone_bandwidth_used)
        .map(|a| f64::from(a.value()))
        .unwrap_or(0.0)
}

pub fn types(fit: &Fit) -> Vec<i32> {
    let mut out: Vec<i32> = Vec::new();
    for item in &fit.items {
        if item.slot == Slot::DroneBay && !out.contains(&item.type_id) {
            out.push(item.type_id);
        }
    }
    out
}

pub fn first_index(fit: &Fit, type_id: i32) -> Option<usize> {
    fit.items
        .iter()
        .position(|it| it.slot == Slot::DroneBay && it.type_id == type_id)
}

pub fn indexes(fit: &Fit, type_id: i32) -> Vec<usize> {
    fit.items
        .iter()
        .enumerate()
        .filter(|(_, it)| it.slot == Slot::DroneBay && it.type_id == type_id)
        .map(|(i, _)| i)
        .collect()
}

pub fn stack(fit: &Fit, type_id: i32) -> Stack {
    let mut out = Stack::default();
    for item in &fit.items {
        if item.slot != Slot::DroneBay || item.type_id != type_id {
            continue;
        }
        if item.state == State::Offline {
            out.stowed += item.quantity;
        } else {
            out.launched += item.quantity;
        }
    }
    out
}

pub fn load(ids: &AttrIds, calc: &Calculation) -> Load {
    Load {
        bay_used: ship_attr(calc, ids.drone_capacity_load),
        bay_total: ship_attr(calc, ids.drone_capacity),
        bandwidth_used: ship_attr(calc, ids.drone_bandwidth_load),
        bandwidth_total: ship_attr(calc, ids.drone_bandwidth),
        launched: ship_attr(calc, ids.drone_active).max(0.0) as u32,
        max_launched: character_attr(calc, ids.max_active_drones).max(0.0) as u32,
    }
}

fn fits_in(free: f64, each: f64) -> u32 {
    if each <= 0.0 {
        return u32::MAX;
    }
    if free <= 0.0 {
        return 0;
    }
    (free / each + 1e-9).floor().clamp(0.0, f64::from(u32::MAX)) as u32
}

pub fn room(info: &impl Info, ids: &AttrIds, calc: &Calculation, type_id: i32) -> Room {
    let load = load(ids, calc);
    let bay = fits_in(load.bay_total - load.bay_used, volume(info, type_id));
    let by_bandwidth = fits_in(
        load.bandwidth_total - load.bandwidth_used,
        bandwidth(info, ids, type_id),
    );
    let by_count = load.max_launched.saturating_sub(load.launched);
    Room {
        bay,
        launch: by_bandwidth.min(by_count),
    }
}

pub fn plan_launched(stack: Stack, room: Room, want: u32) -> Stack {
    let launched = want
        .min(stack.total())
        .min(stack.launched.saturating_add(room.launch));
    Stack {
        launched,
        stowed: stack.total() - launched,
    }
}

pub fn plan_total(stack: Stack, room: Room, want: u32) -> Stack {
    let total = want.min(stack.total().saturating_add(room.bay));
    let added = total.saturating_sub(stack.total()).min(room.launch);
    let launched = stack.launched.saturating_add(added).min(total);
    Stack {
        launched,
        stowed: total - launched,
    }
}

pub fn plan_pack(stack: Stack, room: Room) -> Stack {
    let add = room.launch.min(room.bay);
    if add > 0 {
        return Stack {
            launched: stack.launched + add,
            stowed: stack.stowed,
        };
    }
    if room.bay > 0 {
        return Stack {
            launched: stack.launched,
            stowed: stack.stowed + 1,
        };
    }
    stack
}

pub fn plan_fill(stack: Stack, room: Room) -> Stack {
    Stack {
        launched: stack.launched,
        stowed: stack.stowed + room.bay,
    }
}

pub fn set_stack(fit: &mut Fit, type_id: i32, wanted: Stack) {
    let parts: Vec<(u32, State)> = [
        (wanted.launched, State::Active),
        (wanted.stowed, State::Offline),
    ]
    .into_iter()
    .filter(|(quantity, _)| *quantity > 0)
    .collect();

    let mut places = indexes(fit, type_id);
    let template = places.first().map(|i| fit.items[*i].clone());

    for (n, (quantity, state)) in parts.iter().enumerate() {
        match places.get(n) {
            Some(i) => {
                let item = &mut fit.items[*i];
                item.quantity = *quantity;
                item.state = *state;
            }
            None => {
                let at = places.last().map_or(fit.items.len(), |i| i + 1);
                let mut item = template
                    .clone()
                    .unwrap_or_else(|| new_item(type_id, Slot::DroneBay, *state));
                item.quantity = *quantity;
                item.state = *state;
                fit.items.insert(at, item);
                places.push(at);
            }
        }
    }

    for i in places.into_iter().skip(parts.len()).rev() {
        fit.items.remove(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOMY: Room = Room { bay: 20, launch: 5 };

    #[test]
    fn a_pack_launches_what_the_hull_allows() {
        let out = plan_pack(Stack::default(), ROOMY);
        assert_eq!(
            out,
            Stack {
                launched: 5,
                stowed: 0
            }
        );
    }

    #[test]
    fn a_pack_with_no_bandwidth_left_leaves_one_spare() {
        let room = Room { bay: 20, launch: 0 };
        let out = plan_pack(Stack::default(), room);
        assert_eq!(
            out,
            Stack {
                launched: 0,
                stowed: 1
            }
        );
    }

    #[test]
    fn a_pack_with_a_full_bay_adds_nothing() {
        let room = Room { bay: 0, launch: 5 };
        assert_eq!(plan_pack(Stack::default(), room), Stack::default());
    }

    #[test]
    fn launching_moves_drones_out_of_the_bay_and_keeps_the_total() {
        let stack = Stack {
            launched: 1,
            stowed: 4,
        };
        let room = Room { bay: 0, launch: 4 };
        let out = plan_launched(stack, room, 3);
        assert_eq!(
            out,
            Stack {
                launched: 3,
                stowed: 2
            }
        );
    }

    #[test]
    fn launching_stops_at_the_limit() {
        let stack = Stack {
            launched: 2,
            stowed: 8,
        };
        let room = Room { bay: 0, launch: 3 };
        let out = plan_launched(stack, room, 10);
        assert_eq!(
            out,
            Stack {
                launched: 5,
                stowed: 5
            }
        );
    }

    #[test]
    fn carrying_more_fills_the_bay_and_launches_what_it_can() {
        let stack = Stack {
            launched: 5,
            stowed: 0,
        };
        let room = Room { bay: 10, launch: 0 };
        let out = plan_total(stack, room, 8);
        assert_eq!(
            out,
            Stack {
                launched: 5,
                stowed: 3
            }
        );
    }

    #[test]
    fn carrying_fewer_recalls_the_drones_that_are_gone() {
        let stack = Stack {
            launched: 5,
            stowed: 0,
        };
        let out = plan_total(stack, Room::default(), 3);
        assert_eq!(
            out,
            Stack {
                launched: 3,
                stowed: 0
            }
        );
    }

    #[test]
    fn carrying_stops_at_the_bay() {
        let stack = Stack {
            launched: 0,
            stowed: 2,
        };
        let room = Room { bay: 1, launch: 0 };
        assert_eq!(plan_total(stack, room, 9).total(), 3);
    }

    #[test]
    fn a_fit_without_the_drone_grows_a_stack() {
        let mut fit = crate::edit::new_fit(17843, Default::default());
        set_stack(
            &mut fit,
            2456,
            Stack {
                launched: 5,
                stowed: 3,
            },
        );
        assert_eq!(fit.items.len(), 2);
        assert_eq!((fit.items[0].state, fit.items[0].quantity), (State::Active, 5));
        assert_eq!(
            (fit.items[1].state, fit.items[1].quantity),
            (State::Offline, 3)
        );
    }

    #[test]
    fn a_stack_that_runs_dry_leaves_no_item_behind() {
        let mut fit = crate::edit::new_fit(17843, Default::default());
        set_stack(
            &mut fit,
            2456,
            Stack {
                launched: 5,
                stowed: 3,
            },
        );
        set_stack(
            &mut fit,
            2456,
            Stack {
                launched: 2,
                stowed: 0,
            },
        );
        assert_eq!(fit.items.len(), 1);
        set_stack(&mut fit, 2456, Stack::default());
        assert!(fit.items.is_empty());
    }

    #[test]
    fn drones_keep_their_place_among_the_other_items() {
        let mut fit = crate::edit::new_fit(17843, Default::default());
        set_stack(
            &mut fit,
            2456,
            Stack {
                launched: 1,
                stowed: 0,
            },
        );
        fit.items
            .push(new_item(2185, Slot::DroneBay, State::Active));
        set_stack(
            &mut fit,
            2456,
            Stack {
                launched: 1,
                stowed: 2,
            },
        );
        assert_eq!(fit.items[0].type_id, 2456);
        assert_eq!(fit.items[1].type_id, 2456);
        assert_eq!(fit.items[2].type_id, 2185);
    }
}
