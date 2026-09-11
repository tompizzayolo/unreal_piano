use crate::UnrealPianoParams;
use crate::presets::PRESETS;
use nice_plug::context::gui::ParamSetter;
use nice_plug::{editor::dpi::LogicalSize, prelude::*};
use nice_plug_egui::{
    EguiEditorState, EguiNiceSettings, NiceEguiApp, RepaintNotifier, create_egui_editor,
    resizable_window::ResizableWindow, widgets,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

pub const MIN_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(920.0, 520.0);
pub const INITIAL_WINDOW_SIZE: LogicalSize<f32> = LogicalSize::new(980.0, 660.0);
pub const RESIZE_HINT: ResizeHint = ResizeHint::resizable().with_min_logical_size(MIN_WINDOW_SIZE);
pub const INITIAL_SCALE_FACTOR: f32 = 1.0;

pub struct EditorSharedState {
    pub params: Arc<UnrealPianoParams>,
    pub peak_meter: Arc<AtomicF32>,
    pub active_voice_count: Arc<AtomicU32>,
}

pub struct PianoEditor {
    open_state: Option<OpenEditorState>,
    params: Arc<UnrealPianoParams>,
    peak_meter: Arc<AtomicF32>,
    active_voice_count: Arc<AtomicU32>,
    zoom_factor: f32,
}

struct OpenEditorState {
    nice_gui_ctx: nice_plug::context::gui::GuiContext,
}

impl PianoEditor {
    pub fn new(shared: EditorSharedState, zoom_factor: f32) -> Self {
        Self {
            open_state: None,
            params: shared.params,
            peak_meter: shared.peak_meter,
            active_voice_count: shared.active_voice_count,
            zoom_factor,
        }
    }
}

impl NiceEguiApp for PianoEditor {
    fn build(
        &mut self,
        _egui_ctx: egui::Context,
        nice_gui_ctx: nice_plug::context::gui::GuiContext,
        _frame: &mut nice_plug_egui::Frame,
    ) -> Result<(), nice_plug_egui::baseview::HandlerError> {
        self.open_state = Some(OpenEditorState { nice_gui_ctx });
        Ok(())
    }

    fn editor_closed(&mut self) {
        self.open_state = None;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut nice_plug_egui::Frame) {
        let Some(state) = self.open_state.as_mut() else {
            return;
        };
        let setter = state.nice_gui_ctx.param_setter();

        ResizableWindow::new("unreal-piano-window")
            .min_size(egui::vec2(MIN_WINDOW_SIZE.width, MIN_WINDOW_SIZE.height))
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::same(16))
                    .show(ui, |ui| {
                        ui.heading("Unreal Piano");
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            for preset in PRESETS.iter() {
                                if ui.button(preset.name).clicked() {
                                    for assignment in preset.values.iter() {
                                        let param = assignment.parameter.param(&self.params);
                                        setter.begin_set_parameter(param);
                                        setter.set_parameter_normalized(
                                            param,
                                            param.preview_normalized(assignment.value),
                                        );
                                        setter.end_set_parameter(param);
                                    }
                                }
                            }
                        });

                        ui.add_space(12.0);

                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(200.0);
                                ui.label(egui::RichText::new("Output").strong());
                                ui.add_space(4.0);
                                param_row(ui, "Gain", &self.params.output_gain, &setter);
                                param_row(
                                    ui,
                                    "Min strike",
                                    &self.params.minimum_strike_velocity,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Max strike",
                                    &self.params.maximum_strike_velocity,
                                    &setter,
                                );
                                param_row(ui, "Damper", &self.params.damper_release_ms, &setter);

                                ui.add_space(8.0);
                                let peak_meter_db = nice_plug::util::gain_to_db(
                                    self.peak_meter.load(Ordering::Relaxed),
                                );
                                let mut peak_meter_normalized = (peak_meter_db + 60.0) / 60.0;
                                if peak_meter_normalized <= 0.0 {
                                    peak_meter_normalized = 0.0;
                                }
                                let peak_text =
                                    if peak_meter_db > nice_plug::util::MINUS_INFINITY_DB {
                                        format!("{:.1} dBFS", peak_meter_db)
                                    } else {
                                        String::from("-inf dBFS")
                                    };
                                ui.add(
                                    egui::ProgressBar::new(peak_meter_normalized).text(peak_text),
                                );
                                ui.label(format!(
                                    "voices: {}",
                                    self.active_voice_count.load(Ordering::Relaxed)
                                ));
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.set_width(200.0);
                                ui.label(egui::RichText::new("Voicing").strong());
                                ui.add_space(4.0);
                                param_row(
                                    ui,
                                    "Hammer hardness",
                                    &self.params.hammer_hardness,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Hardness piano",
                                    &self.params.hammer_hardness_piano,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Hardness mezzo",
                                    &self.params.hammer_hardness_mezzo,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Hardness forte",
                                    &self.params.hammer_hardness_forte,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Noise (min)",
                                    &self.params.hammer_noise_min,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Noise (max)",
                                    &self.params.hammer_noise_max,
                                    &setter,
                                );
                                param_row(ui, "Hammer tone", &self.params.hammer_tone, &setter);
                                param_row(ui, "Soft pedal", &self.params.soft_pedal, &setter);
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.set_width(200.0);
                                ui.label(egui::RichText::new("Tuning").strong());
                                ui.add_space(4.0);
                                param_row(
                                    ui,
                                    "Unison (min)",
                                    &self.params.unison_width_min,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Unison (max)",
                                    &self.params.unison_width_max,
                                    &setter,
                                );
                            });

                            ui.separator();

                            ui.vertical(|ui| {
                                ui.set_width(200.0);
                                ui.label(egui::RichText::new("Design").strong());
                                ui.add_space(4.0);
                                param_row(ui, "String length", &self.params.string_length, &setter);
                                param_row(ui, "Strike point", &self.params.strike_point, &setter);
                                param_row(
                                    ui,
                                    "Sympathetic res.",
                                    &self.params.sympathetic_resonance,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Duplex scale",
                                    &self.params.duplex_scale_resonance,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Blooming energy",
                                    &self.params.blooming_energy,
                                    &setter,
                                );
                                param_row(
                                    ui,
                                    "Blooming inertia",
                                    &self.params.blooming_inertia,
                                    &setter,
                                );
                            });
                        });

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            ui.label("scale");
                            let before = self.zoom_factor;
                            egui::ComboBox::from_id_salt("zoom_factor")
                                .selected_text(format!(
                                    "{}%",
                                    (self.zoom_factor * 100.0).round() as u32
                                ))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.zoom_factor, 0.75, "75%");
                                    ui.selectable_value(&mut self.zoom_factor, 1.0, "100%");
                                    ui.selectable_value(&mut self.zoom_factor, 1.25, "125%");
                                    ui.selectable_value(&mut self.zoom_factor, 1.5, "150%");
                                    ui.selectable_value(&mut self.zoom_factor, 1.75, "175%");
                                    ui.selectable_value(&mut self.zoom_factor, 2.0, "200%");
                                });
                            if self.zoom_factor != before {
                                ui.set_zoom_factor(self.zoom_factor);
                            }
                        });
                    });
            });
    }

    fn track_info_changed(&mut self, _info: TrackInfo) {}
}

fn param_row(ui: &mut egui::Ui, label: &str, param: &FloatParam, setter: &ParamSetter) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(widgets::ParamSlider::for_param(param, setter));
    });
}

pub fn build_editor(
    editor_state: Arc<EguiEditorState>,
    repaint_notifier: RepaintNotifier,
    editor: PianoEditor,
) -> Option<nice_plug_egui::EguiEditor<PianoEditor>> {
    create_egui_editor(
        editor_state,
        repaint_notifier,
        EguiNiceSettings::new().with_resize_hint(RESIZE_HINT),
        editor,
    )
}
