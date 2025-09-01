use crate::params::builder::ParamGroupBuilder;
use crate::params::ParamGroup;

/// Implementations
/// Points
pub fn build_default_point_params() -> ParamGroup {
    ParamGroupBuilder::new("Point Distribution")
        .enum_param(
            "Distribution",
            vec!["Random Uniform".into(), "Uniform Grid".into()],
            0,
        )
        	.with_tooltip("Changes point distribution algorithm")
        .int_param("Point Count", 1000, 10, 5000, 50)
        	.with_tooltip("How many points to generate")
        .float_param("Point Radius", 2.0, 0.5, 10.0, 0.5)
        	.with_tooltip("Circle size of each point")
        .build()
}

pub fn build_default_voronoi_params() -> ParamGroup {
	ParamGroupBuilder::new("Voronoi")
		.int_param("Lloyd Relaxations", 1, 1, 10, 1)
			.with_tooltip("Set iterations of applied Lloyd relaxations")
		.build()
}

pub fn build_default_landmass_params() -> ParamGroup {
	ParamGroupBuilder::new("Landmass")
		.float_param("Scale", 1., 0.1, 10., 0.1)
		.float_param("X Size", 150., 50., 1000., 10.)
		.float_param("Y Size", 150., 50., 1000., 10.)
		.float_param("Rotation", 0., 0., 360., 5.)
		.float_param("Water Level", 0., -1., 1., 0.01)
		.float_param("Elevation Multiplier", 1., 0.25, 2.5, 0.25)
		.float_param("Noise Scale", 0.02, 0.001, 0.1, 0.001)
		.float_param("Noise Amplitude", 0.5, 0., 2., 0.1)
		.build()
}