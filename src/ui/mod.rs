//! Iced editor: preset buttons, a horizontal row of parameter sections
//! (Output / Voicing / Tuning / Design), a peak meter and the voice count.
use crate::presets::{AdjustableParameter, PRESETS};
use nice_plug::{editor::dpi::LogicalSize, prelude::*};
use nice_plug_iced::{
    IcedNiceContext, PersistentState,
    iced::{
        self, Center, Element, Subscription, Theme,
        widget::{Column, ProgressBar, Row, button, column, pick_list, row, slider, text},
    },
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::UnrealPianoParams;

pub const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(920.0, 520.0);
pub const INITIAL_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(980.0, 660.0);
pub const RESIZE_HINT: ResizeHint = ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);
pub const INITIAL_SCALE_FACTOR: f32 = 1.0;

/// Fixed width of each parameter section.
const SECTION_WIDTH: f32 = 200.0;

/// State shared between the editor and the audio thread.
pub struct EditorSharedState {
    pub params: Arc<UnrealPianoParams>,
    pub peak_meter: Arc<AtomicF32>,
    pub active_voice_count: Arc<AtomicU32>,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Poll,
    WindowResized,
    SetScaleFactor(f32),
    ParameterChanged {
        which: AdjustableParameter,
        normalized_value: f32,
    },
    PresetApplied {
        preset_index: usize,
    },
}

pub struct PianoGui {
    persistent_state: PersistentState<EditorSharedState>,
    ctx: IcedNiceContext,
    peak_meter_db: f32,
}

/// A labeled slider bound to one parameter.
fn parameter_control(
    label: &str,
    param: &FloatParam,
    which: AdjustableParameter,
) -> Column<'static, Message> {
    let readout = param.normalized_value_to_string(param.modulated_normalized_value(), true);
    column![
        text(format!("{}: {}", label, readout)),
        slider(
            0.0..=1.0,
            param.modulated_normalized_value(),
            move |normalized_value| Message::ParameterChanged {
                which,
                normalized_value
            }
        )
        .step(0.001f32),
    ]
    .spacing(4.0)
    .width(SECTION_WIDTH)
}

impl PianoGui {
    pub fn new(persistent_state: PersistentState<EditorSharedState>, ctx: IcedNiceContext) -> Self {
        Self {
            persistent_state,
            ctx,
            peak_meter_db: nice_plug::util::gain_to_db(0.0),
        }
    }

    pub fn theme(&self) -> Option<Theme> {
        Some(Theme::Dark)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            iced::poll_events().map(|_| Message::Poll),
            iced::window_resized().map(|_| Message::WindowResized),
        ])
    }

    pub fn scale_factor(&self) -> f32 {
        self.ctx.user_scale_factor()
    }

    pub fn update(&mut self, message: Message) {
        let setter = self.ctx.nice_context.param_setter();
        let params = &self.persistent_state.params;

        match message {
            Message::Poll => {
                self.peak_meter_db = nice_plug::util::gain_to_db(
                    self.persistent_state.peak_meter.load(Ordering::Relaxed),
                );
            }
            Message::SetScaleFactor(scale_factor) => {
                self.ctx.set_user_scale_factor(scale_factor);
            }
            Message::WindowResized => {
                self.ctx.sync_window_size();
            }
            Message::ParameterChanged {
                which,
                normalized_value,
            } => {
                let param = which.param(params);
                setter.begin_set_parameter(param);
                setter.set_parameter_normalized(param, normalized_value);
                setter.end_set_parameter(param);
            }
            Message::PresetApplied { preset_index } => {
                if let Some(preset) = PRESETS.get(preset_index) {
                    for assignment in preset.values.iter() {
                        let param = assignment.parameter.param(params);
                        setter.begin_set_parameter(param);
                        setter.set_parameter_normalized(
                            param,
                            param.preview_normalized(assignment.value),
                        );
                        setter.end_set_parameter(param);
                    }
                }
            }
        }
    }

    pub fn view(&self) -> Column<'_, Message> {
        let params = &self.persistent_state.params;

        let scale_options = [
            ScaleOption(0.75),
            ScaleOption(1.0),
            ScaleOption(1.25),
            ScaleOption(1.5),
            ScaleOption(1.75),
            ScaleOption(2.0),
        ];

        let mut preset_buttons: Vec<Element<'_, Message>> = Vec::new();
        for (preset_index, preset) in PRESETS.iter().enumerate() {
            preset_buttons.push(
                button(preset.name)
                    .on_press(Message::PresetApplied { preset_index })
                    .into(),
            );
        }
        let preset_row = Row::with_children(preset_buttons).spacing(8.0);

        // The four parameter sections, side by side.
        let output_section = column![
            text("Output").size(16),
            parameter_control("Gain", &params.output_gain, AdjustableParameter::OutputGain),
            parameter_control(
                "Min strike",
                &params.minimum_strike_velocity,
                AdjustableParameter::MinimumStrike
            ),
            parameter_control(
                "Max strike",
                &params.maximum_strike_velocity,
                AdjustableParameter::MaximumStrike
            ),
            parameter_control(
                "Damper",
                &params.damper_release_ms,
                AdjustableParameter::Damper
            ),
            ProgressBar::new(-60.0..=0.0, self.peak_meter_db.max(-60.0)),
            text(format!(
                "voices: {}",
                self.persistent_state
                    .active_voice_count
                    .load(Ordering::Relaxed)
            )),
        ]
        .spacing(10.0);

        let voicing_section = column![
            text("Voicing").size(16),
            parameter_control(
                "Hammer hardness",
                &params.hammer_hardness,
                AdjustableParameter::HammerHardness
            ),
            parameter_control(
                "Hardness piano",
                &params.hammer_hardness_piano,
                AdjustableParameter::HammerHardnessPiano
            ),
            parameter_control(
                "Hardness mezzo",
                &params.hammer_hardness_mezzo,
                AdjustableParameter::HammerHardnessMezzo
            ),
            parameter_control(
                "Hardness forte",
                &params.hammer_hardness_forte,
                AdjustableParameter::HammerHardnessForte
            ),
            parameter_control(
                "Noise (min)",
                &params.hammer_noise_min,
                AdjustableParameter::HammerNoiseMinimum
            ),
            parameter_control(
                "Noise (max)",
                &params.hammer_noise_max,
                AdjustableParameter::HammerNoiseMaximum
            ),
            parameter_control(
                "Hammer tone",
                &params.hammer_tone,
                AdjustableParameter::HammerTone
            ),
            parameter_control(
                "Soft pedal",
                &params.soft_pedal,
                AdjustableParameter::SoftPedal
            ),
        ]
        .spacing(10.0);

        let tuning_section = column![
            text("Tuning").size(16),
            parameter_control(
                "Unison (min)",
                &params.unison_width_min,
                AdjustableParameter::UnisonWidthMinimum
            ),
            parameter_control(
                "Unison (max)",
                &params.unison_width_max,
                AdjustableParameter::UnisonWidthMaximum
            ),
        ]
        .spacing(10.0);

        let design_section = column![
            text("Design").size(16),
            parameter_control(
                "String length",
                &params.string_length,
                AdjustableParameter::StringLength
            ),
            parameter_control(
                "Strike point",
                &params.strike_point,
                AdjustableParameter::StrikePoint
            ),
            parameter_control(
                "Sympathetic res.",
                &params.sympathetic_resonance,
                AdjustableParameter::SympatheticResonance
            ),
            parameter_control(
                "Duplex scale",
                &params.duplex_scale_resonance,
                AdjustableParameter::DuplexScale
            ),
            parameter_control(
                "Blooming energy",
                &params.blooming_energy,
                AdjustableParameter::BloomingEnergy
            ),
            parameter_control(
                "Blooming inertia",
                &params.blooming_inertia,
                AdjustableParameter::BloomingInertia
            ),
        ]
        .spacing(10.0);

        column![
            text("Unreal Piano").size(24),
            text("physical modeling after arXiv:2409.03481").size(12),
            preset_row,
            row![
                output_section,
                voicing_section,
                tuning_section,
                design_section,
            ]
            .spacing(24.0),
            row![
                text("scale"),
                pick_list(
                    scale_options,
                    Some(ScaleOption(self.ctx.user_scale_factor())),
                    |option| Message::SetScaleFactor(option.0)
                )
            ]
            .align_y(Center)
            .spacing(7.0),
        ]
        .padding(20)
        .spacing(16.0)
        .align_x(Center)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ScaleOption(f32);

impl std::fmt::Display for ScaleOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}%", (self.0 * 100.0).round())
    }
}
