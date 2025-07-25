use std::any::Any;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ParamGroup {
    pub title: String,
    pub enabled: bool,
    pub params: Vec<Param>,
}

impl ParamGroup {
    pub fn new<T: Into<String>>(title: T, params: Vec<Param>) -> Self {
        Self {
            title: title.into(),
            enabled: true,
            params,
        }
    }

    /// Draws all parameter controls inside a collapsible UI section.
    /// Returns true if any parameter was visibly changed.
    pub fn draw_controls(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;

        ui.collapsing(&self.title, |ui| {
            ui.checkbox(&mut self.enabled, "Enabled");

            if self.enabled {
                for param in self.params.iter_mut() {
                    changed |= param.draw(ui);
                }
            }
        });

        changed
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_param(&self, name: &str) -> Option<&Param> {
        self.params.iter().find(|p| p.name == name)
    }

    pub fn get_param_mut(&mut self, name: &str) -> Option<&mut Param> {
        self.params.iter_mut().find(|p| p.name == name)
    }
}

// bool param
#[derive(Debug)]
pub struct BoolParam {
    pub val: bool,
}

impl ParamValue for BoolParam {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.checkbox(&mut self.val, "").changed()
    }

    fn as_str(&self) -> &str {
        panic!("BoolParam has no string value")
    }

    fn as_bool(&self) -> &bool {
        &self.val
    }

    fn as_int(&self) -> &usize {
        panic!("BoolParam has no int value")
    }

    fn as_float(&self) -> &f32 {
        panic!("BoolParam has no float value")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// float param
#[derive(Debug)]
pub struct FloatParam {
    pub val: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

impl ParamValue for FloatParam {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.add(
            egui::Slider::new(&mut self.val, self.min..=self.max)
                .step_by(self.step as f64)
                .text(""),
        ).changed()
    }

    fn as_str(&self) -> &str {
        panic!("FloatParam has no string value")
    }
    fn as_bool(&self) -> &bool {
        panic!("FloatParam has no bool value")
    }
    fn as_int(&self) -> &usize {
        panic!("FloatParam has no int value")
    }
    fn as_float(&self) -> &f32 {
        &self.val
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// usize param
#[derive(Debug)]
pub struct IntParam {
    pub val: usize,
    pub min: usize,
    pub max: usize,
    pub step: usize,
}

impl ParamValue for IntParam {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.add(
            egui::Slider::new(&mut self.val, self.min..=self.max)
                .step_by(self.step as f64)
                .text(""),
        ).changed()
    }

    fn as_str(&self) -> &str {
        panic!("IntParam has no string value")
    }

    fn as_bool(&self) -> &bool {
        panic!("IntParam has no bool value")
    }

    fn as_int(&self) -> &usize {
        &self.val
    }

    fn as_float(&self) -> &f32 {
        panic!("IntParam has no float value")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// string param
#[derive(Debug)]
pub struct StringParam {
    pub val: String,
}

impl ParamValue for StringParam {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool {
        ui.text_edit_singleline(&mut self.val).changed()
    }

    fn as_str(&self) -> &str {
        &self.val
    }

    fn as_bool(&self) -> &bool {
        panic!("StringParam has no bool value")
    }

    fn as_int(&self) -> &usize {
        panic!("StringParam has no int value")
    }

    fn as_float(&self) -> &f32 {
        panic!("StringParam has no float value")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// enum param
#[derive(Debug)]
pub struct EnumParam {
	pub options: Vec<String>,
	pub selected: usize,
}

impl ParamValue for EnumParam {
	fn draw(&mut self, ui: &mut egui::Ui) -> bool {
		let mut changed = false;
		let current = self.selected;

        egui::ComboBox::from_id_source(ui.id().with("enum_param"))
            .selected_text(&self.options[current])
            .show_ui(ui, |ui| {
                for (i, opt) in self.options.iter().enumerate() {
                    if ui.selectable_label(self.selected == i, opt).clicked() {
                        self.selected = i;
                        changed = true;
                    }
                }
            });
		changed
	}

	fn as_str(&self) -> &str {
		&self.options[self.selected]
	}

	fn as_bool(&self) -> &bool {
		panic!("EnumParam has no bool value")
	}

	fn as_int(&self) -> &usize {
        &self.selected
    }

    fn as_float(&self) -> &f32 {
        panic!("EnumParam has no float value")
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

//////////////////////////////////////////////////////////////////////
/// The core trait for any parameter value (f32, usize, bool, etc.) //
//////////////////////////////////////////////////////////////////////

pub trait ParamValue: Debug + Send {
    fn draw(&mut self, ui: &mut egui::Ui) -> bool;

    fn as_str(&self) -> &str;
    fn as_bool(&self) -> &bool;
    fn as_int(&self) -> &usize;
    fn as_float(&self) -> &f32;

    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
pub struct Param {
    pub name: String,
    pub tooltip: Option<String>,
    pub value: Box<dyn ParamValue>,
}

impl Param {
    pub fn draw(&mut self, ui: &mut egui::Ui) -> bool {
		let mut changed = false;
        ui.horizontal(|ui| {
            let label = egui::RichText::new(&self.name).strong();
            if let Some(tt) = &self.tooltip {
                ui.label(label).on_hover_text(tt);
            } else {
                ui.label(label);
            }

            changed = self.value.draw(ui);
        });
		changed
    }

    pub fn as_float(&self) -> Option<f32> {
        self.value
            .as_any()
            .downcast_ref::<FloatParam>()
            .map(|p| p.val)
    }

    pub fn as_int(&self) -> Option<usize> {
        self.value
            .as_any()
            .downcast_ref::<IntParam>()
            .map(|p| p.val)
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.value
            .as_any()
            .downcast_ref::<BoolParam>()
            .map(|p| p.val)
    }
}

pub mod builder;
pub mod util;
