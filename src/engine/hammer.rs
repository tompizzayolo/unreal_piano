use super::vector::Vector3;

pub const GRAVITATIONAL_ACCELERATION: f64 = 9.81;
const REST_RAIL_STIFFNESS: f64 = 2.0e3;
const REST_RAIL_DAMPING: f64 = 1.3;
const CONTACT_VALIDITY_MULTIPLIER: f64 = 3.0;

#[derive(Clone, Copy)]
pub struct HammerGeometry {
    pub horizontal_arm_length: f64,
    pub vertical_arm_length: f64,
    pub felt_thickness: f64,
    pub string_distance_from_pivot: f64,
    pub string_angle_from_pivot: f64,
    pub string_inclination_angle: f64,
    pub shank_line_density: f64,
    pub pivot_damping: f64,
}

#[derive(Clone, Copy)]
pub struct FeltProperties {
    pub stiffness: [f64; 3],
    pub nonlinearity_exponent: [f64; 3],
    pub relaxation_time: [f64; 3],
}

pub struct Hammer {
    pub geometry: HammerGeometry,
    pub felt: FeltProperties,
    pub shank_mass: f64,
    pub moment_of_inertia: f64,
    pub pivot_to_center_of_mass_distance: f64,
    pub center_of_mass_angle: f64,
    pub rest_angle: f64,
    pub rotation_angle: f64,
    pub angular_velocity: f64,
    string_rest_x: f64,
    string_rest_z: f64,
    string_inclination_sin: f64,
    string_inclination_cos: f64,
}

#[inline]
fn rotate_into_felt_frame(vector: Vector3, rotation_angle: f64) -> Vector3 {
    let (sine_part, cosine_part) = rotation_angle.sin_cos();
    Vector3::new(
        cosine_part * vector.x + sine_part * vector.z,
        vector.y,
        -sine_part * vector.x + cosine_part * vector.z,
    )
}

#[inline]
fn rotate_from_felt_frame(vector: Vector3, rotation_angle: f64) -> Vector3 {
    let (sine_part, cosine_part) = rotation_angle.sin_cos();
    Vector3::new(
        cosine_part * vector.x - sine_part * vector.z,
        vector.y,
        sine_part * vector.x + cosine_part * vector.z,
    )
}

#[inline]
fn rotate_string_to_hammer_frame(vector: Vector3, sine_part: f64, cosine_part: f64) -> Vector3 {
    Vector3::new(
        cosine_part * vector.x - sine_part * vector.z,
        vector.y,
        sine_part * vector.x + cosine_part * vector.z,
    )
}

#[inline]
fn rotate_hammer_to_string_frame(vector: Vector3, sine_part: f64, cosine_part: f64) -> Vector3 {
    Vector3::new(
        cosine_part * vector.x + sine_part * vector.z,
        vector.y,
        -sine_part * vector.x + cosine_part * vector.z,
    )
}

#[inline]
fn powered_magnitude(value: f64, exponent: f64) -> f64 {
    let magnitude = value.abs();
    if exponent == 1.0 {
        magnitude
    } else if exponent == 2.0 {
        magnitude * magnitude
    } else {
        magnitude.powf(exponent)
    }
}

#[inline]
fn felt_axis_force(
    compression: f64,
    reference: f64,
    stiffness: f64,
    exponent: f64,
    relaxation_time: f64,
    step_size: f64,
) -> f64 {
    if compression == 0.0 {
        return 0.0;
    }

    let current = powered_magnitude(compression, exponent);
    let previous = powered_magnitude(reference, exponent);
    let rate = if step_size > 0.0 {
        (current - previous) / step_size
    } else {
        0.0
    };

    let magnitude = stiffness * current + relaxation_time * stiffness * rate;
    if magnitude <= 0.0 {
        return 0.0;
    }

    -compression.signum() * magnitude
}

impl Hammer {
    #[inline]
    pub fn new(geometry: HammerGeometry, felt: FeltProperties) -> Self {
        let horizontal_arm_length = geometry.horizontal_arm_length;
        let vertical_arm_length = geometry.vertical_arm_length;
        let horizontal_arm_length_squared = horizontal_arm_length * horizontal_arm_length;
        let vertical_arm_length_squared = vertical_arm_length * vertical_arm_length;
        let total_arm_length = horizontal_arm_length + vertical_arm_length;

        let shank_mass = geometry.shank_line_density * total_arm_length;
        let moment_of_inertia = geometry.shank_line_density
            * (horizontal_arm_length_squared * horizontal_arm_length / 3.0
                + vertical_arm_length_squared * vertical_arm_length / 3.0
                + horizontal_arm_length_squared * vertical_arm_length);

        let (center_along_horizontal_arm, center_along_vertical_arm) = if total_arm_length > 0.0 {
            (
                (0.5 * horizontal_arm_length_squared + horizontal_arm_length * vertical_arm_length)
                    / total_arm_length,
                0.5 * vertical_arm_length_squared / total_arm_length,
            )
        } else {
            (0.0, 0.0)
        };

        let (string_inclination_sin, string_inclination_cos) =
            geometry.string_inclination_angle.sin_cos();
        let (string_angle_sin, string_angle_cos) = geometry.string_angle_from_pivot.sin_cos();

        Hammer {
            geometry,
            felt,
            shank_mass,
            moment_of_inertia,
            pivot_to_center_of_mass_distance: (center_along_horizontal_arm
                * center_along_horizontal_arm
                + center_along_vertical_arm * center_along_vertical_arm)
                .sqrt(),
            center_of_mass_angle: center_along_vertical_arm.atan2(center_along_horizontal_arm),
            rest_angle: 0.0,
            rotation_angle: 0.0,
            angular_velocity: 0.0,
            string_rest_x: geometry.string_distance_from_pivot * string_angle_cos,
            string_rest_z: geometry.string_distance_from_pivot * string_angle_sin,
            string_inclination_sin,
            string_inclination_cos,
        }
    }

    #[inline]
    fn felt_frame_position_of_string_point(
        &self,
        rotation_angle: f64,
        string_displacement_at_strike_point: Vector3,
        lateral_offset: f64,
    ) -> Vector3 {
        let string_rest_position =
            Vector3::new(self.string_rest_x, lateral_offset, self.string_rest_z);

        let position_in_static_hammer_frame = string_rest_position
            + rotate_string_to_hammer_frame(
                string_displacement_at_strike_point,
                self.string_inclination_sin,
                self.string_inclination_cos,
            );

        Vector3::new(
            -self.geometry.horizontal_arm_length,
            0.0,
            -self.geometry.vertical_arm_length,
        ) + rotate_into_felt_frame(position_in_static_hammer_frame, rotation_angle)
    }

    #[inline]
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

    #[inline]
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
        let validity_limit = CONTACT_VALIDITY_MULTIPLIER * self.geometry.felt_thickness;

        if compression.dot(compression) > validity_limit * validity_limit {
            return (Vector3::ZERO, false);
        }

        (compression, true)
    }

    #[inline]
    pub fn felt_contact_force(
        &self,
        compression: Vector3,
        compression_reference_for_rate_term: Vector3,
        step_size: f64,
    ) -> Vector3 {
        Vector3::new(
            felt_axis_force(
                compression.x,
                compression_reference_for_rate_term.x,
                self.felt.stiffness[0],
                self.felt.nonlinearity_exponent[0],
                self.felt.relaxation_time[0],
                step_size,
            ),
            felt_axis_force(
                compression.y,
                compression_reference_for_rate_term.y,
                self.felt.stiffness[1],
                self.felt.nonlinearity_exponent[1],
                self.felt.relaxation_time[1],
                step_size,
            ),
            felt_axis_force(
                compression.z,
                compression_reference_for_rate_term.z,
                self.felt.stiffness[2],
                self.felt.nonlinearity_exponent[2],
                self.felt.relaxation_time[2],
                step_size,
            ),
        )
    }

    #[inline]
    pub fn force_on_string_in_string_frame(
        &self,
        felt_frame_force: Vector3,
        rotation_angle: f64,
    ) -> Vector3 {
        let force_in_static_hammer_frame = rotate_from_felt_frame(felt_frame_force, rotation_angle);
        rotate_hammer_to_string_frame(
            force_in_static_hammer_frame,
            self.string_inclination_sin,
            self.string_inclination_cos,
        )
    }

    #[inline]
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

    #[inline]
    pub fn rest_rail_torque(&self) -> f64 {
        let rail_penetration = self.rest_angle - self.rotation_angle;
        if rail_penetration <= 0.0 {
            return 0.0;
        }

        let torque =
            REST_RAIL_STIFFNESS * rail_penetration - REST_RAIL_DAMPING * self.angular_velocity;
        if torque > 0.0 { torque } else { 0.0 }
    }

    #[inline]
    pub fn total_torque(&self, total_felt_force_on_strings: Vector3) -> f64 {
        self.shank_torque(self.rotation_angle, total_felt_force_on_strings)
            + self.rest_rail_torque()
    }

    #[inline]
    pub fn advance_shank_rotation(&mut self, total_torque: f64, step_size: f64) {
        let angular_acceleration_contribution = step_size * total_torque / self.moment_of_inertia;
        let damping_amplification =
            1.0 + step_size * self.geometry.pivot_damping / self.moment_of_inertia;

        self.angular_velocity =
            (self.angular_velocity + angular_acceleration_contribution) / damping_amplification;
        self.rotation_angle += step_size * self.angular_velocity;
    }

    #[inline]
    pub fn advance_shank_with_felt_force(
        &mut self,
        total_felt_force_on_strings: Vector3,
        step_size: f64,
    ) {
        let total_torque = self.total_torque(total_felt_force_on_strings);
        self.advance_shank_rotation(total_torque, step_size);
    }

    pub fn resting_contact_angle(&self, lateral_offset: f64) -> f64 {
        let mut lower_bound = -0.5;
        let mut upper_bound = 0.5;

        for _ in 0..40 {
            let clearance = self
                .felt_frame_position_of_string_point(lower_bound, Vector3::ZERO, lateral_offset)
                .z
                - self.geometry.felt_thickness;

            if clearance <= 0.0 {
                lower_bound -= 0.25;
            } else {
                break;
            }
        }

        for _ in 0..40 {
            let clearance = self
                .felt_frame_position_of_string_point(upper_bound, Vector3::ZERO, lateral_offset)
                .z
                - self.geometry.felt_thickness;

            if clearance >= 0.0 {
                upper_bound += 0.25;
            } else {
                break;
            }
        }

        for _ in 0..200 {
            let midpoint = 0.5 * (lower_bound + upper_bound);
            let clearance = self
                .felt_frame_position_of_string_point(midpoint, Vector3::ZERO, lateral_offset)
                .z
                - self.geometry.felt_thickness;

            if clearance > 0.0 {
                lower_bound = midpoint;
            } else {
                upper_bound = midpoint;
            }
        }

        0.5 * (lower_bound + upper_bound)
    }

    #[inline]
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
