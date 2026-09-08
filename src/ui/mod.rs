//! Iced editor: output gain, hammer velocity range, damper release, a peak
//! meter and the active voice count.

use nice_plug::{editor::dpi::LogicalSize, prelude::*};
use nice_plug_iced::{
    IcedNiceContext, PersistentState,
    iced::{
        self, Center, PollSubNotifier, Subscription, Theme,
        widget::{Column, ProgressBar, column, pick_list, row, slider, text},
    },
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::UnrealPianoParams;

pub const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(360.0, 560.0);
pub const RESIZE_HINT: ResizeHint = ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);
pub const INITIAL_SCALE_FACTOR: f32 = 1.0;

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
    GainChanged(f32),
    MinimumStrikeChanged(f32),
    MaximumStrikeChanged(f32),
    DamperReleaseChanged(f32),
}

pub struct PianoGui {
    persistent_state: PersistentState<EditorSharedState>,
    ctx: IcedNiceContext,
    peak_meter_db: f32,
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
            Message::GainChanged(normalized_value) => {
                setter.begin_set_parameter(&params.output_gain);
                setter.set_parameter_normalized(&params.output_gain, normalized_value);
                setter.end_set_parameter(&params.output_gain);
            }
            Message::MinimumStrikeChanged(normalized_value) => {
                setter.begin_set_parameter(&params.minimum_strike_velocity);
                setter.set_parameter_normalized(&params.minimum_strike_velocity, normalized_value);
                setter.end_set_parameter(&params.minimum_strike_velocity);
            }
            Message::MaximumStrikeChanged(normalized_value) => {
                setter.begin_set_parameter(&params.maximum_strike_velocity);
                setter.set_parameter_normalized(&params.maximum_strike_velocity, normalized_value);
                setter.end_set_parameter(&params.maximum_strike_velocity);
            }
            Message::DamperReleaseChanged(normalized_value) => {
                setter.begin_set_parameter(&params.damper_release_ms);
                setter.set_parameter_normalized(&params.damper_release_ms, normalized_value);
                setter.end_set_parameter(&params.damper_release_ms);
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

        column![
            text("Unreal Piano").size(24),
            text("physical modeling after arXiv:2409.03481").size(12),
            text(format!(
                "Gain: {}",
                params.output_gain.normalized_value_to_string(
                    params.output_gain.modulated_normalized_value(),
                    true
                )
            )),
            slider(
                0.0..=1.0,
                params.output_gain.modulated_normalized_value(),
                Message::GainChanged
            )
            .step(0.001f32),
            text(format!(
                "Min strike: {:.2} m/s",
                params.minimum_strike_velocity.value()
            )),
            slider(
                0.0..=1.0,
                params.minimum_strike_velocity.modulated_normalized_value(),
                Message::MinimumStrikeChanged
            )
            .step(0.001f32),
            text(format!(
                "Max strike: {:.2} m/s",
                params.maximum_strike_velocity.value()
            )),
            slider(
                0.0..=1.0,
                params.maximum_strike_velocity.modulated_normalized_value(),
                Message::MaximumStrikeChanged
            )
            .step(0.001f32),
            text(format!(
                "Damper: {:.0} ms",
                params.damper_release_ms.value()
            )),
            slider(
                0.0..=1.0,
                params.damper_release_ms.modulated_normalized_value(),
                Message::DamperReleaseChanged
            )
            .step(0.001f32),
            ProgressBar::new(-60.0..=0.0, self.peak_meter_db.max(-60.0)),
            text(format!(
                "voices: {}",
                self.persistent_state
                    .active_voice_count
                    .load(Ordering::Relaxed)
            )),
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
        .spacing(12.0)
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
