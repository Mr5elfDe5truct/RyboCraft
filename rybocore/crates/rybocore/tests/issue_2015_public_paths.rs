//! Compile-only external contract for the public paths requested in issue #2015.

/// Names supported by `topology::brep`.
pub mod topology_brep {
    pub use rybocore::topology::brep::{
        api_brep_copy, copy_faces, create_face_from_surface, create_faces_with_curves,
        create_parallel_projection_curves, create_planar_face_with_3d_curves,
        create_planar_faces_with_3d_curves, delete_face, edges, faces, is_manifold_solid, is_sheet,
        make_edge_in_face, make_topology_from_data, make_vertex_loop,
        merge_curves_on_surface_through_surface, new_brep, orient_sheet_surfaces,
        orient_trimmed_surfaces, regions, remove_topological_edges_and_vertices, shell_vertices,
        stitch_and_orient, stitch_faces, topology_ref, with_kernel_callbacks, ArenaNotifyState,
        DeleteFaceOptions, EdgeInFace, FaceBuildEnv, MakeTopologyOptions,
        ParallelProjectionOptions, StitchAndOrientOptions, TopologyEnv, HEALER_VERSION,
    };
}

/// Names supported by `topology::face`.
pub mod topology_face {
    pub use rybocore::topology::face::{
        delete_inner_loop_with_kernel_callbacks, edges, faceuses, loops, surface, tolerance,
        uv_domain, PPF_AREA, PPF_AREA_FIRST_MOMENTS,
    };
}

/// Names supported by `topology::edgeuse`.
pub mod topology_edgeuse {
    pub use rybocore::topology::edgeuse::{
        evaluate_binormal, get_or_create_uv_trim_curve, BinormalOptions,
    };
}

/// Names supported by `nurbs::b_spline_surface_n_lib`.
pub mod nurbs_b_spline_surface_n_lib {
    pub use rybocore::nurbs::b_spline_surface_n_lib::{EvaluateFlags, OrientType};
}

/// Names supported by `blend::fillet_executive`.
pub mod blend_fillet_executive {
    pub use rybocore::blend::fillet_executive::{FilletLaw, FilletLawKind, NoRoomReason};
}

/// Names supported by `blend::fillet_geom`.
pub mod blend_fillet_geom {
    pub use rybocore::blend::fillet_geom::{FilletCornerType, FilletErrorType};
}

/// Names supported by `blend::fillet_solver`.
pub mod blend_fillet_solver {
    pub use rybocore::blend::fillet_solver::{FilletSurfaceGenerator, FilletSurfaceGeneratorType};
}

/// Names supported by `offset::offset_executive`.
pub mod offset_offset_executive {
    pub use rybocore::offset::offset_executive::{
        api_offset_brep_full, api_shell_brep_full, OffsetNotifications, OffsetOptions,
        ShellCopyIdentities, ShellOptions,
    };
}

/// Names supported by `surface::surface`.
pub mod surface_surface {
    pub use rybocore::surface::surface::{SurfaceIntersection, TsectCurveType};
}

/// Names supported by `curve::curve`.
pub mod curve_curve {
    pub use rybocore::curve::curve::{compute_crvs_normal, ApproximateCurveOptions};
}

/// Names supported by `geom::geom_utility`.
pub mod geom_geom_utility {
    pub use rybocore::geom::geom_utility::ContinuityType;
}

/// Names supported by `primitive::primitive_creation`.
pub mod primitive_primitive_creation {
    pub use rybocore::primitive::primitive_creation::{
        create_rectangle, Boolean2d, PrimitiveCreation,
    };
}

/// Names supported by `heal::stitch`.
pub mod heal_stitch {
    pub use rybocore::heal::stitch::api_stitch_into_solid;
}

/// Names supported by `query::face`.
pub mod query_face {
    pub use rybocore::query::face::{calculate_bounding_box, calculate_shell_bounding_box};
}

/// Names supported by `nurbs::b_spline_curve`.
pub mod nurbs_b_spline_curve {
    pub use rybocore::nurbs::b_spline_curve::{BSplineCurve, KnotType};
}

/// Names supported by `nurbs::b_spline_surface`.
pub mod nurbs_b_spline_surface {
    pub use rybocore::nurbs::b_spline_surface::{BSplineSurface, SurfParamType};
}

/// Names supported by `surface::cone`.
pub mod surface_cone {
    pub use rybocore::surface::cone::Cone;
}

/// Names supported by `surface::plane`.
pub mod surface_plane {
    pub use rybocore::surface::plane::Plane;
}

/// Convenience entry points and predicate types promised alongside the exports.
pub mod convenience {
    pub use rybocore::blend::fillet_executive::session::FilletSession;
    pub use rybocore::mass::integrator::properties::{
        brep_mass_properties_default, brep_volume_default,
    };
    pub use rybocore::query::face_surface_intersect;
    pub use rybocore::topology::shell::is_inner_shell_of_tri;
    pub use rybocore::Tri;
}

const _: fn(&rybocore::topology::Region) -> rybocore::Result<rybocore::Tri> =
    rybocore::topology::Region::void_state;
/// Type-check the lifetime-generic session method without executing preparation.
pub fn prepare_two_offsets_compiles(
    session: &mut rybocore::blend::fillet_executive::session::FilletSession<'_>,
    kernel: &rybocore::Kernel,
) -> rybocore::Result<()> {
    session.prepare_per_edge_two_offsets(kernel, &[], &[], &[], &[])
}
