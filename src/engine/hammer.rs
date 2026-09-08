//! Hammer model (paper §6.1): a rigid shank rotating about its pivot and a
//! nonlinear hysteretic felt that contacts the string one-sidedly.
//!
//! Contact geometry (eqs. (6.1)–(6.3)):
//!   x1'  = r0 + R0 u0                (string point in the static hammer frame)
//!   x1'' = [-L1, 0, -L2] + R(θ) x1'  (string point in the rotating felt frame)
//!   contact iff z1'' <= L4 ;  ϑ = x1'' − contact_reference
//!
//! Felt force (eq. (6.4)), per axis of the x''y''z'' frame:
//!   F_i = -sgn(ϑ_i) [ k_i |ϑ_i|^{p_i} + r_i k_i d/dt(|ϑ_i|^{p_i}) ]
//!
//! Sign note: the paper writes the shank ODE (6.6) as
//!   I θ̈ = -μ θ̇ + m g L0 cos(β0+θ) + l·F(d'')
//! but with the rotation conventions of eq. (6.1) (increasing θ raises the
//! hammer head, rotations about -y) the torque signs must be inverted for
//! the dynamics to be physical: gravity must *lower* the head and the string
//! must *push the hammer back*.  This implementation uses
//!   I θ̈ = -μ θ̇ - m g L0 cos(β0+θ) + l·F''
//! with F'' the force on the *string*.
//!
//! Rest rail: the paper does not model the key action.  A real piano's
//! backcheck stops the rebounded hammer a few centimetres below the string
//! and holds it there while the key is down.  Without an equivalent
//! one-sided stop, the shank is a free pendulum under gravity (≈ 97 rad/s²
//! here): it swings ~200° around its pivot within ~0.25 s and re-enters the
//! contact geometry from an orientation in which the one-sided felt model is
//! meaningless (|ϑ| of centimetres → ~10⁹ N → NaN).  `rest_rail_torque`
//! provides that stop.

use super::vector::Vector3;

pub const GRAVITATIONAL_ACCELERATION: f64 = 9.81;

/// Rotational stiffness of the rest rail / backcheck [N·m/rad].  A
/// rebounding shank penetrates it by well under a milliradian, and the
/// rail's own frequency (≈ √(k/I) ≈ 3 kHz) stays far inside the explicit
/// stability limit even at the full audio-rate step.
const REST_RAIL_STIFFNESS: f64 = 2.0e3;

/// Rotational damping of the rest rail [N·m·s/rad], near critical
/// (2·√(k·I) ≈ 1.3): the hammer lands on the rail and stays there.
const REST_RAIL_DAMPING: f64 = 1.3;

/// Rigid-body and contact geometry of the hammer (paper fig. 5 / eq. (6.1)).
#[derive(Clone, Copy)]
pub struct HammerGeometry {
    /// L1 = |P1 P3|: horizontal arm of the bent shank, pivot -> felt base.
    pub horizontal_arm_length: f64,
    /// L2 = |P2 P3|: vertical arm of the bent shank (perpendicular to L1).
    pub vertical_arm_length: f64,
    /// L4 = |P2 P4|: static thickness of the hammer felt.
    pub felt_thickness: f64,
    /// L = |Q0 P1|: distance from the pivot to the (resting) contact point.
    pub string_distance_from_pivot: f64,
    /// α1: angle of P1->Q0 above the hammer frame's x' axis.
    pub string_angle_from_pivot: f64,
    /// α0: inclination of the string's x axis inside the hammer's x'z' plane.
    pub string_inclination_angle: f64,
    /// ρ: line density of the (uniform, bent) shank [kg/m].
    pub shank_line_density: f64,
    /// μ: simplified pivot damping torque coefficient [N·m·s/rad].
    pub pivot_damping: f64,
}

/// Hysteretic felt properties (paper eq. (6.4)), per felt-frame axis
/// (x'' = tangential along the string, y'' = tangential lateral,
///  z'' = normal, i.e. the compression direction).
#[derive(Clone, Copy)]
pub struct FeltProperties {
    /// k_i: felt stiffness [N/m^p].
    pub stiffness: [f64; 3],
    /// p_i: nonlinearity exponent of the compression law.
    pub nonlinearity_exponent: [f64; 3],
    /// r_i: relaxation time of the hysteretic (dissipative) term [s].
    pub relaxation_time: [f64; 3],
}

/// The complete hammer: rigid shank state plus felt law.
pub struct Hammer {
    pub geometry: HammerGeometry,
    pub felt: FeltProperties,
    /// m = ρ (L1 + L2): total shank mass [kg].
    pub shank_mass: f64,
    /// I = ρ (L1³/3 + L2³/3 + L1² L2): moment of inertia about the pivot
    /// (paper eq. (6.5)) [kg·m²].
    pub moment_of_inertia: f64,
    /// L0: distance from pivot to the centre of mass.
    pub pivot_to_center_of_mass_distance: f64,
    /// β0: angle of P1->P0 relative to the horizontal arm.
    pub center_of_mass_angle: f64,
    /// θ_rest: resting angle of the shank against the action's rest rail,
    /// assigned by the owner once the grazing-contact angle is known (see
    /// `rest_rail_torque` for why it must exist).  The default of 0.0 is
    /// already below any contact angle and therefore fails safe.
    pub rest_angle: f64,
    /// θ: shank rotation angle (increasing θ raises the hammer head).
    pub rotation_angle: f64,
    /// θ̇: shank angular velocity.
    pub angular_velocity: f64,
}

/// R(θ) of eq. (6.1): static hammer frame -> rotating felt frame.
fn rotate_into_felt_frame(vector: Vector3, rotation_angle: f64) -> Vector3 {
    let (sine_part, cosine_part) = rotation_angle.sin_cos();
    Vector3::new(
        cosine_part * vector.x + sine_part * vector.z,
        vector.y,
        -sine_part * vector.x + cosine_part * vector.z,
    )
}

/// S(θ) = R(θ)^T of eq. (6.2): rotating felt frame -> static hammer frame.
fn rotate_from_felt_frame(vector: Vector3, rotation_angle: f64) -> Vector3 {
    let (sine_part, cosine_part) = rotation_angle.sin_cos();
    Vector3::new(
        cosine_part * vector.x - sine_part * vector.z,
        vector.y,
        sine_part * vector.x + cosine_part * vector.z,
    )
}

/// R0 of eq. (6.1): string xyz frame -> static hammer frame.
fn rotate_string_to_hammer_frame(vector: Vector3, inclination_angle: f64) -> Vector3 {
    let (sine_part, cosine_part) = inclination_angle.sin_cos();
    Vector3::new(
        cosine_part * vector.x - sine_part * vector.z,
        vector.y,
        sine_part * vector.x + cosine_part * vector.z,
    )
}

/// R0^T: static hammer frame -> string xyz frame.
fn rotate_hammer_to_string_frame(vector: Vector3, inclination_angle: f64) -> Vector3 {
    let (sine_part, cosine_part) = inclination_angle.sin_cos();
    Vector3::new(
        cosine_part * vector.x + sine_part * vector.z,
        vector.y,
        -sine_part * vector.x + cosine_part * vector.z,
    )
}

impl Hammer {
    pub fn new(geometry: HammerGeometry, felt: FeltProperties) -> Self {
        let shank_mass = geometry.shank_line_density
            * (geometry.horizontal_arm_length + geometry.vertical_arm_length);
        let moment_of_inertia = geometry.shank_line_density
            * (geometry.horizontal_arm_length.powi(3) / 3.0
                + geometry.vertical_arm_length.powi(3) / 3.0
                + geometry.horizontal_arm_length
                    * geometry.horizontal_arm_length
                    * geometry.vertical_arm_length);
        let total_arm_length = geometry.horizontal_arm_length + geometry.vertical_arm_length;
        let center_along_horizontal_arm = (0.5 * geometry.horizontal_arm_length.powi(2)
            + geometry.horizontal_arm_length * geometry.vertical_arm_length)
            / total_arm_length;
        let center_along_vertical_arm =
            0.5 * geometry.vertical_arm_length.powi(2) / total_arm_length;
        Hammer {
            geometry,
            felt,
            shank_mass,
            moment_of_inertia,
            pivot_to_center_of_mass_distance: (center_along_horizontal_arm.powi(2)
                + center_along_vertical_arm.powi(2))
            .sqrt(),
            center_of_mass_angle: center_along_vertical_arm.atan2(center_along_horizontal_arm),
            rest_angle: 0.0,
            rotation_angle: 0.0,
            angular_velocity: 0.0,
        }
    }

    /// Position of the string contact point in the rotating felt frame,
    /// i.e. x1'' of eq. (6.1).  `string_displacement_at_strike_point` is the
    /// string's (u, v, w) at the strike point, in the string's own frame.
    fn felt_frame_position_of_string_point(
        &self,
        rotation_angle: f64,
        string_displacement_at_strike_point: Vector3,
        lateral_offset: f64,
    ) -> Vector3 {
        let geometry = &self.geometry;
        // r0: resting position of the contact point in the static hammer frame.
        let string_rest_position = Vector3::new(
            geometry.string_distance_from_pivot * geometry.string_angle_from_pivot.cos(),
            lateral_offset,
            geometry.string_distance_from_pivot * geometry.string_angle_from_pivot.sin(),
        );
        // x1' = r0 + R0 u0.
        let position_in_static_hammer_frame = string_rest_position
            + rotate_string_to_hammer_frame(
                string_displacement_at_strike_point,
                geometry.string_inclination_angle,
            );
        // x1'' = [-L1, 0, -L2] + R(θ) x1'.
        let felt_frame_origin_offset = Vector3::new(
            -geometry.horizontal_arm_length,
            0.0,
            -geometry.vertical_arm_length,
        );
        felt_frame_origin_offset
            + rotate_into_felt_frame(position_in_static_hammer_frame, rotation_angle)
    }

    /// Grazing-contact reference for one string: the position, in the rotating
    /// felt frame, of the string point at the instant the felt first touches
    /// it (string at rest, shank at `contact_angle`).  The felt frame is fixed
    /// to the shank, so this surface point is constant for the whole
    /// simulation.
    ///
    /// Eq. (6.3) subtracts only [0, 0, L4] from x1'', which presumes the
    /// x''y'' origin coincides with the contact point.  In the frame produced
    /// by eq. (6.1) the string point sits ~1.4 cm away along x'' at contact,
    /// so the raw tangential components are dominated by static geometry
    /// rather than felt shear; the power law (6.4) then produces tens of
    /// kilonewtons and the simulation diverges.  Subtracting the grazing
    /// reference restores the intended meaning:
    ///   ϑ_tangential = felt shear (string slip relative to the hammer),
    ///   ϑ_normal     = penetration below the undeformed felt surface.
    ///
    /// `lateral_misalignment` offsets the reference laterally — the hammer
    /// never touches all three unison strings perfectly centered — and seeds
    /// the horizontal polarization (the "horizontal interaction force" of
    /// §6.1.1).
    pub fn contact_reference_in_felt_frame(
        &self,
        contact_angle: f64,
        lateral_offset: f64,
        lateral_misalignment: f64,
    ) -> Vector3 {
        let grazing_position =
            self.felt_frame_position_of_string_point(contact_angle, Vector3::ZERO, lateral_offset);
        Vector3::new(
            grazing_position.x,
            grazing_position.y + lateral_misalignment,
            self.geometry.felt_thickness,
        )
    }

    /// Felt compression ϑ and contact flag (paper eq. (6.3)), measured
    /// relative to `contact_reference_in_felt_frame`.  Contact is detected on
    /// the normal axis alone: z1'' <= L4.
    ///
    /// Validity guard: the one-sided law treats the felt as a half-space,
    /// which is only meaningful while the string point stays near the
    /// grazing reference (real compressions/shears are sub-millimetre).  If
    /// the geometries have interpenetrated by more than the felt's physical
    /// extent — e.g. a shank that has swung around its pivot — the model has
    /// left its domain, and we report no contact rather than compute
    /// meaningless forces.
    pub fn felt_compression(
        &self,
        rotation_angle: f64,
        string_displacement_at_strike_point: Vector3,
        lateral_offset: f64,
        contact_reference_in_felt_frame: Vector3,
    ) -> (Vector3, bool) {
        let position_in_felt_frame = self.felt_frame_position_of_string_point(
            rotation_angle,
            string_displacement_at_strike_point,
            lateral_offset,
        );
        if position_in_felt_frame.z > self.geometry.felt_thickness {
            return (Vector3::ZERO, false);
        }
        let compression = position_in_felt_frame - contact_reference_in_felt_frame;
        let distance_from_reference = compression.dot(compression).sqrt();
        if distance_from_reference > 3.0 * self.geometry.felt_thickness {
            return (Vector3::ZERO, false);
        }
        (compression, true)
    }

    /// Hysteretic felt force (paper eq. (6.4)) acting on the *string*,
    /// expressed in the rotating felt frame x''y''z''.
    ///
    /// d/dt |ϑ|^{p} is approximated by the finite difference
    /// (|ϑ|^{p} - |ϑ_reference|^{p}) / step_size.
    pub fn felt_contact_force(
        &self,
        compression: Vector3,
        compression_reference_for_rate_term: Vector3,
        step_size: f64,
    ) -> Vector3 {
        let felt = &self.felt;
        let compression_components = [compression.x, compression.y, compression.z];
        let reference_components = [
            compression_reference_for_rate_term.x,
            compression_reference_for_rate_term.y,
            compression_reference_for_rate_term.z,
        ];
        let mut force_components = [0.0; 3];
        for axis in 0..3 {
            let compression_this_step = compression_components[axis];
            if compression_this_step == 0.0 {
                continue; // one-sided contact: no penetration, no force
            }
            let compression_magnitude = compression_this_step
                .abs()
                .powf(felt.nonlinearity_exponent[axis]);
            let reference_magnitude = reference_components[axis]
                .abs()
                .powf(felt.nonlinearity_exponent[axis]);
            let rate_of_compression_law = (compression_magnitude - reference_magnitude) / step_size;
            force_components[axis] = -compression_this_step.signum()
                * (felt.stiffness[axis] * compression_magnitude
                    + felt.relaxation_time[axis] * felt.stiffness[axis] * rate_of_compression_law);
        }
        Vector3::new(
            force_components[0],
            force_components[1],
            force_components[2],
        )
    }

    /// F'' -> string frame: F^(a) = R0^T S(θ) F'' (paper §6.1.1).
    pub fn force_on_string_in_string_frame(
        &self,
        felt_frame_force: Vector3,
        rotation_angle: f64,
    ) -> Vector3 {
        let force_in_static_hammer_frame = rotate_from_felt_frame(felt_frame_force, rotation_angle);
        rotate_hammer_to_string_frame(
            force_in_static_hammer_frame,
            self.geometry.string_inclination_angle,
        )
    }

    /// Total torque about the shank rotation axis (corrected eq. (6.6);
    /// see the module documentation for the sign fix).
    ///
    /// l = [L2, 0, -L1] is the lever arm of the felt-force application point
    /// (the felt-frame origin, which sits at [L1, 0, L2] from the pivot in
    /// felt-frame coordinates).
    pub fn shank_torque(&self, rotation_angle: f64, total_felt_force_on_strings: Vector3) -> f64 {
        let lever_arm = Vector3::new(
            self.geometry.vertical_arm_length,
            0.0,
            -self.geometry.horizontal_arm_length,
        );
        let felt_reaction_torque = lever_arm.dot(total_felt_force_on_strings);
        let gravity_torque = -self.shank_mass
            * GRAVITATIONAL_ACCELERATION
            * self.pivot_to_center_of_mass_distance
            * (self.center_of_mass_angle + rotation_angle).cos();
        gravity_torque + felt_reaction_torque
    }

    /// One-sided torque of the rest rail: the rebounded hammer must be
    /// caught a few centimetres below the string, as a real piano's
    /// backcheck does.  Engages only while `rotation_angle < rest_angle`;
    /// near-critically damped so the hammer lands and stays.
    pub fn rest_rail_torque(&self) -> f64 {
        let rail_penetration = self.rest_angle - self.rotation_angle;
        if rail_penetration <= 0.0 {
            return 0.0;
        }
        REST_RAIL_STIFFNESS * rail_penetration - REST_RAIL_DAMPING * self.angular_velocity
    }

    /// Semi-implicit update of the shank rotation ODE (paper §7.2.3):
    /// the pivot damping is treated implicitly, the torques explicitly.
    pub fn advance_shank_rotation(&mut self, total_torque: f64, step_size: f64) {
        let angular_acceleration_contribution = step_size * total_torque / self.moment_of_inertia;
        let damping_amplification =
            1.0 + step_size * self.geometry.pivot_damping / self.moment_of_inertia;
        self.angular_velocity =
            (self.angular_velocity + angular_acceleration_contribution) / damping_amplification;
        self.rotation_angle += step_size * self.angular_velocity;
    }

    /// Bisection for the angle at which the felt first grazes the resting
    /// string (the paper's t = 0 initial condition, §6.1.1).
    pub fn resting_contact_angle(&self, lateral_offset: f64) -> f64 {
        let felt_surface_clearance = |rotation_angle: f64| -> f64 {
            self.felt_frame_position_of_string_point(rotation_angle, Vector3::ZERO, lateral_offset)
                .z
                - self.geometry.felt_thickness
        };
        // Widen the bracket if the defaults do not contain the root.
        let mut lower_bound = -0.5;
        let mut upper_bound = 0.5;
        for _ in 0..40 {
            if felt_surface_clearance(lower_bound) <= 0.0 {
                lower_bound -= 0.25;
            } else {
                break;
            }
        }
        for _ in 0..40 {
            if felt_surface_clearance(upper_bound) >= 0.0 {
                upper_bound += 0.25;
            } else {
                break;
            }
        }
        for _ in 0..200 {
            let midpoint = 0.5 * (lower_bound + upper_bound);
            if felt_surface_clearance(midpoint) > 0.0 {
                lower_bound = midpoint;
            } else {
                upper_bound = midpoint;
            }
        }
        0.5 * (lower_bound + upper_bound)
    }

    /// Rate at which the felt compression grows per radian of shank rotation
    /// (= -d z1''/dθ > 0); used to convert a target hammer speed into the
    /// initial angular velocity.
    pub fn compression_rate_per_radian(&self, contact_angle: f64, lateral_offset: f64) -> f64 {
        let angle_increment = 1.0e-4;
        let clearance_above = self
            .felt_frame_position_of_string_point(
                contact_angle + angle_increment,
                Vector3::ZERO,
                lateral_offset,
            )
            .z;
        let clearance_below = self
            .felt_frame_position_of_string_point(
                contact_angle - angle_increment,
                Vector3::ZERO,
                lateral_offset,
            )
            .z;
        -(clearance_above - clearance_below) / (2.0 * angle_increment)
    }
}
