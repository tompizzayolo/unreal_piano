use nice_plug::editor::dpi::LogicalSize;
use nice_plug::prelude::*;
use nice_plug_iced::{
    IcedEditor, IcedEditorState, IcedNiceContext, IcedNiceSettings, PersistentState,
    iced::{
        self, Center, PollSubNotifier, Subscription, Theme,
        widget::{column, slider, text, toggler},
    },
};
use nice_plug_iced::{application, create_iced_editor};
use std::sync::Arc;

use crate::params::PianoParams;

pub const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(400.0, 550.0);
pub const RESIZE_HINT: ResizeHint = ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);
pub const INITIAL_SCALE_FACTOR: f32 = 1.0;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Poll,
    WindowResized,
    ParamChanged(ParamId, f32),
    SustainToggled(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamId {
    OutGain,
    HammerStiffness,
    HammerExponent,
    HammerHysteresis,
    StringDecay,
    Inharmonicity,
    SbCoupling,
    SbDecay,
}

pub struct PianoEditorState {
    pub params: Arc<PianoParams>,
}

pub struct PianoGui {
    persistent_state: PersistentState<PianoEditorState>,
    ctx: IcedNiceContext,
}

impl PianoGui {
    pub fn new(persistent_state: PersistentState<PianoEditorState>, ctx: IcedNiceContext) -> Self {
        Self {
            persistent_state,
            ctx,
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
            Message::Poll => {}
            Message::WindowResized => {
                self.ctx.sync_window_size();
            }
            Message::ParamChanged(id, value) => {
                let param = match id {
                    ParamId::OutGain => &params.out_gain,
                    ParamId::HammerStiffness => &params.hammer_stiffness,
                    ParamId::HammerExponent => &params.hammer_exponent,
                    ParamId::HammerHysteresis => &params.hammer_hysteresis,
                    ParamId::StringDecay => &params.string_decay,
                    ParamId::Inharmonicity => &params.inharmonicity,
                    ParamId::SbCoupling => &params.sb_coupling,
                    ParamId::SbDecay => &params.sb_decay,
                };
                setter.begin_set_parameter(param);
                setter.set_parameter_normalized(param, value);
                setter.end_set_parameter(param);
            }
            Message::SustainToggled(value) => {
                setter.begin_set_parameter(&params.sustain);
                setter.set_parameter(&params.sustain, if value { 1.0 } else { 0.0 });
                setter.end_set_parameter(&params.sustain);
            }
        }
    }

    pub fn view(&self) -> iced::widget::Column<'_, Message> {
        let params = &self.persistent_state.params;

        column![
            text("Physical Piano").size(24),
            self.param_slider("Output Gain", &params.out_gain, ParamId::OutGain),
            self.param_slider(
                "Hammer Stiffness",
                &params.hammer_stiffness,
                ParamId::HammerStiffness
            ),
            self.param_slider(
                "Hammer Exponent",
                &params.hammer_exponent,
                ParamId::HammerExponent
            ),
            self.param_slider(
                "Hammer Hysteresis",
                &params.hammer_hysteresis,
                ParamId::HammerHysteresis
            ),
            self.param_slider("String Decay", &params.string_decay, ParamId::StringDecay),
            self.param_slider(
                "Inharmonicity",
                &params.inharmonicity,
                ParamId::Inharmonicity
            ),
            self.param_slider(
                "Soundboard Coupling",
                &params.sb_coupling,
                ParamId::SbCoupling
            ),
            self.param_slider("Soundboard Decay", &params.sb_decay, ParamId::SbDecay),
            toggler(params.sustain.value() >= 0.5)
                .label("Sustain Pedal")
                .on_toggle(Message::SustainToggled),
        ]
        .padding(20)
        .spacing(12.0)
        .align_x(Center)
    }

    fn param_slider<'a>(
        &self,
        label: &'a str,
        param: &'a FloatParam,
        id: ParamId,
    ) -> iced::widget::Column<'a, Message> {
        column![
            text(format!(
                "{}: {}",
                label,
                param.normalized_value_to_string(param.modulated_normalized_value(), true)
            )),
            slider(0.0..=1.0, param.modulated_normalized_value(), move |v| {
                Message::ParamChanged(id, v)
            })
            .step(0.001f32),
        ]
        .spacing(4.0)
        .width(iced::Length::Fill)
    }
}

pub fn create_editor(
    params: Arc<PianoParams>,
    editor_state: Arc<IcedEditorState>,
    notifier: PollSubNotifier,
) -> Option<IcedEditor> {
    create_iced_editor(
        editor_state,
        PianoEditorState { params },
        notifier,
        IcedNiceSettings::new().with_resize_hint(RESIZE_HINT),
        |editor_state, nice_ctx| {
            Ok(application(
                editor_state,
                nice_ctx,
                PianoGui::new,
                PianoGui::update,
                PianoGui::view,
            )
            .theme(PianoGui::theme)
            .scale_factor(PianoGui::scale_factor)
            .subscription(PianoGui::subscription)
            .run())
        },
    )
}
