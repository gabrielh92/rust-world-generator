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

    pub fn int_param(
        mut self,
        name: impl Into<String>,
        val: usize,
        min: usize,
        max: usize,
        step: usize,
    ) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            visual_only: false,
            is_divider: false,
            value: Box::new(IntParam { val, min, max, step }),
        });
        self
    }

    #[allow(dead_code)]
    pub fn bool_param(mut self, name: impl Into<String>, val: bool) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            visual_only: false,
            is_divider: false,
            value: Box::new(BoolParam { val }),
        });
        self
    }

    pub fn float_param(
        mut self,
        name: impl Into<String>,
        val: f32,
        min: f32,
        max: f32,
        step: f32,
    ) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            visual_only: false,
            is_divider: false,
            value: Box::new(FloatParam { val, min, max, step }),
        });
        self
    }

    pub fn enum_param(
        mut self,
        name: impl Into<String>,
        options: Vec<String>,
        selected: usize,
    ) -> Self {
        self.params.push(Param {
            name: name.into(),
            tooltip: None,
            visual_only: false,
            is_divider: false,
            value: Box::new(EnumParam { options, selected }),
        });
        self
    }

    /// Insert a labelled section divider. Renders as a separator + small bold label.
    /// Does not affect param lookup or pipeline re-run logic.
    pub fn with_section(mut self, label: impl Into<String>) -> Self {
        self.params.push(Param {
            name: label.into(),
            tooltip: None,
            visual_only: true,
            is_divider: true,
            value: Box::new(BoolParam { val: false }), // unused
        });
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        if let Some(last) = self.params.last_mut() {
            last.tooltip = Some(tooltip.into());
        }
        self
    }

    /// Mark the last-added param as visual-only (won't trigger pipeline re-run).
    pub fn visual_only(mut self) -> Self {
        if let Some(last) = self.params.last_mut() {
            last.visual_only = true;
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
