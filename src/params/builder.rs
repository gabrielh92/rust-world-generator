use crate::params::{BoolParam, EnumParam, FloatParam, IntParam, Param, ParamGroup};

/// Builder
pub struct ParamGroupBuilder {
    title: String,
    enabled: bool,
    params: Vec<Param>,
}

impl ParamGroupBuilder {
	pub fn new(title: impl Into<String>) -> Self {
		Self {
			title: title.into(),
			enabled: true,
			params: vec![],
		}
	}

    pub fn int_param(mut self, name: impl Into<String>, val: usize, min: usize, max: usize, step: usize) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            value: Box::new(IntParam { val, min, max, step }),
        });
        self
    }

    pub fn bool_param(mut self, name: impl Into<String>, val: bool) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            value: Box::new(BoolParam { val }),
        });
        self
    }

    pub fn float_param(mut self, name: impl Into<String>, val: f32, min: f32, max: f32, step: f32) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            value: Box::new(FloatParam { val, min, max, step }),
        });
        self
    }

	pub fn enum_param(mut self, name: impl Into<String>, options: Vec<String>, selected: usize) -> Self {
		self.params.push(Param {
			name: name.into(),
			tooltip: None,
			value: Box::new(EnumParam { options, selected })
		});
		self
	}

	pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        if let Some(last) = self.params.last_mut() {
            last.tooltip = Some(tooltip.into());
        }
        self
    }

    pub fn build(self) -> ParamGroup {
        ParamGroup {
            title: self.title,
            enabled: self.enabled,
            params: self.params,
        }
    }
}