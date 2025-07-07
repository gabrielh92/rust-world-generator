use crate::params::ParamGroup;
use crate::params::builder::ParamGroupBuilder;

/// Implementations
/// Points
pub fn build_default_point_params() -> ParamGroup {
    ParamGroupBuilder::new("Point Distribution")
        .int_param("Point Count", 1000, 10, 5000, 50)
            .with_tooltip("How many points to generate")
        .float_param("Point Radius", 2.0, 0.5, 10.0, 0.5)
            .with_tooltip("Circle size of each point")
        .build()
}
