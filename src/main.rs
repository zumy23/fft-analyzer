use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use eframe::egui;
use std::sync::{Arc, RwLock};

struct AppState {
    devices: Vec<cpal::Device>,
    selected_device: usize,
    audio_data: Arc<RwLock<Vec<f32>>>,
    _stream: Option<Stream>,
    is_playing: bool,
}

impl AppState {
    fn new() -> Self {
        let host = cpal::default_host();
        let devices: Vec<cpal::Device> = host.input_devices().unwrap().collect();
        AppState {
            devices,
            selected_device: 0,
            audio_data: Arc::new(RwLock::new(Vec::new())),
            _stream: None,
            is_playing: false,
        }
    }

    fn start_stream(&mut self) {
        let device = &self.devices[self.selected_device];
        let config = device.default_input_config().unwrap();
        
        let sample_format = config.sample_format();
        let config = cpal::StreamConfig::from(config);

        let audio_data = self.audio_data.clone();
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let stream = match sample_format {
            SampleFormat::F32 => self.build_input_stream::<f32>(device, &config, err_fn, audio_data),
            SampleFormat::I16 => self.build_input_stream::<i16>(device, &config, err_fn, audio_data),
            SampleFormat::U16 => self.build_input_stream::<u16>(device, &config, err_fn, audio_data),
            _ => panic!("sample format is not supported: {:?}", sample_format),
        };

        stream.play().unwrap();
        self._stream = Some(stream);
        self.is_playing = true;
    }

    fn stop_stream(&mut self) {
        if let Some(stream) = self._stream.take() {
            drop(stream);
        }
        self.is_playing = false;
    }

    fn build_input_stream<T>(
        &self,
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        err_fn: impl Fn(cpal::StreamError) + Send + 'static,
        audio_data: Arc<RwLock<Vec<f32>>>,
    ) -> Stream
    where
        T: cpal::Sample + cpal::SizedSample + Into<f32>,
    {
        let channels = config.channels as usize;
        device.build_input_stream(
            config,
            move |data: &[T], _| {
                let mut buffer = audio_data.write().unwrap();
                for frame in data.chunks(channels) {
                    buffer.push(frame[0].into());
                    if buffer.len() > 2048 {
                        let drain_size = buffer.len() - 2048;
                        buffer.drain(0..drain_size);
                    }
                }
            },
            err_fn,
            None,
        ).unwrap()
    }
}

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Audio Device:");
                if egui::ComboBox::from_label("device_select")
                    .selected_text(self.devices[self.selected_device].name().unwrap_or("Unknown Device".into()))
                    .show_ui(ui, |ui| {
                        for (i, dev) in self.devices.iter().enumerate() {
                            let name = dev.name().unwrap_or("Unknown Device".into());
                            ui.selectable_value(&mut self.selected_device, i, name);
                        }
                    })
                    .response
                    .changed()
                {
                    if self.is_playing {
                        self.stop_stream();
                        self.start_stream();
                    }
                }

                if self.is_playing {
                    if ui.button("Stop").clicked() {
                        self.stop_stream();
                    }
                } else {
                    if ui.button("Start").clicked() {
                        self.start_stream();
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let audio_data = self.audio_data.read().unwrap();
            let points: Vec<[f64; 2]> = audio_data
                .iter()
                .enumerate()
                .map(|(i, &sample)| [i as f64, sample as f64])
                .collect();
            
            egui::plot::Plot::new("waveform_plot")
                .height(ui.available_height())
                .width(ui.available_width())
                .show(ui, |plot_ui| {
                    plot_ui.line(egui::plot::Line::new(
                        egui::plot::PlotPoints::from_iter(points.into_iter()),
                    ));
                });
        });

        ctx.request_repaint();
    }
}

fn main() {
    let mut app = AppState::new();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Audio Waveform Viewer",
        native_options,
        Box::new(|_cc| Box::new(app)),
    );
}
