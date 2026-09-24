// SPDX-License-Identifier: GPL-3.0-only

use crate::{shell::CosmicSurface, state::State, utils::prelude::*};
use smithay::{
    input::pointer::PointerHandle,
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
    wayland::{
        pointer_constraints::{ConstraintRemove, PointerConstraintsHandler},
        seat::WaylandFocus,
    },
};

pub use smithay::wayland::pointer_constraints::{PointerConstraintRef, with_pointer_constraint};

pub fn activate_pointer_constraint(
    surface: &WlSurface,
    pointer: &PointerHandle<State>,
    surface_location: Point<f64, Logical>,
) {
    with_pointer_constraint(surface, pointer, |constraint| {
        if let Some(constraint) = constraint
            && !constraint.is_active()
        {
            let point = (pointer.current_location() - surface_location).to_i32_floor();
            if constraint
                .region()
                .is_none_or(|region| region.contains(point))
            {
                constraint.activate();
            }
        }
    });
}

impl PointerConstraintsHandler for State {
    fn new_constraint(&mut self, surface: &WlSurface, pointer: &PointerHandle<Self>) {
        let Some(seat) = self
            .common
            .shell
            .read()
            .seats
            .iter()
            .find(|s| s.get_pointer().as_ref() == Some(pointer))
            .cloned()
        else {
            return;
        };

        seat.set_pointer_constraint_hint(None);
        let current_output = seat.active_output();
        let position = seat.get_pointer().unwrap().current_location().as_global();
        let shell = self.common.shell.read();
        let under = State::surface_under(position, &current_output, &shell);

        let surface_location = if let Some((target, target_loc)) = under
            && let Some(under_surface) = target.wl_surface()
        {
            if *under_surface == *surface {
                Some(target_loc)
            } else {
                CosmicSurface::surface_tree_offset(surface, &under_surface)
                    .map(|offset| target_loc - offset.to_f64().as_global())
            }
        } else {
            None
        };

        if let Some(surface_location) = surface_location {
            activate_pointer_constraint(surface, pointer, surface_location.as_logical());
        }
    }

    fn remove_constraint(
        &mut self,
        surface: &WlSurface,
        pointer: &PointerHandle<Self>,
        constraint_remove: ConstraintRemove,
    ) {
        match constraint_remove {
            ConstraintRemove::PointerLeave(_) => {
                // If the constraint was broken by the pointer forcibly leaving the surface, then it doesn't
                // make much sense to warp it.
            }
            ConstraintRemove::Destroyed(constraint) => {
                let Some(seat) = self
                    .common
                    .shell
                    .read()
                    .seats
                    .iter()
                    .find(|s| s.get_pointer().as_ref() == Some(pointer))
                    .cloned()
                else {
                    return;
                };
                let Some((hint_surface, hint_location)) = seat.pointer_constraint_hint() else {
                    return;
                };

                if hint_surface == *surface {
                    self.apply_cursor_hint(surface, pointer, hint_location, Some(&constraint));
                    seat.set_pointer_constraint_hint(None);
                }
            }
        }
    }

    fn cursor_position_hint(
        &mut self,
        surface: &WlSurface,
        pointer: &PointerHandle<Self>,
        location: Point<f64, Logical>,
    ) {
        if with_pointer_constraint(surface, pointer, |constraint| {
            constraint.is_some_and(|c| c.is_active())
        }) {
            let seat = self
                .common
                .shell
                .read()
                .seats
                .iter()
                .find(|s| s.get_pointer().as_ref() == Some(pointer))
                .cloned();

            if let Some(seat) = seat {
                seat.set_pointer_constraint_hint(Some((surface.clone(), location)));
            }
        }
    }
}
