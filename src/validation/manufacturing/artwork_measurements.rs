use crate::reports::Finding;
use serde_json::json;

use super::geometry::CopperObjectRef;
use super::insert_optional_copper_feature_owner_measurements;
use crate::board_ir::{LayoutCopperRegion, LayoutCopperSegment};

pub(super) fn insert_unprefixed_solder_mask_object_measurements(
    finding: &mut Finding,
    object: CopperObjectRef<'_>,
) {
    finding
        .measured
        .insert("solder_mask_kind".to_string(), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding
                .measured
                .insert("solder_mask_feature_index".to_string(), json!(index));
            finding.measured.insert(
                "solder_mask_feature_x_mm".to_string(),
                json!(feature.at.x_mm),
            );
            finding.measured.insert(
                "solder_mask_feature_y_mm".to_string(),
                json!(feature.at.y_mm),
            );
            finding.measured.insert(
                "solder_mask_feature_layer".to_string(),
                json!(feature.layer),
            );
            insert_optional_copper_feature_owner_measurements(
                finding,
                "solder_mask_feature",
                feature,
            );
            finding.measured.insert(
                "solder_mask_feature_aperture".to_string(),
                json!(feature.aperture),
            );
            finding.measured.insert(
                "solder_mask_feature_shape".to_string(),
                json!(feature.shape),
            );
            finding.measured.insert(
                "solder_mask_feature_size_x_mm".to_string(),
                json!(feature.size.x_mm),
            );
            finding.measured.insert(
                "solder_mask_feature_size_y_mm".to_string(),
                json!(feature.size.y_mm),
            );
            finding.measured.insert(
                "solder_mask_feature_source_primitive".to_string(),
                json!(feature.source_primitive),
            );
            finding.measured.insert(
                "solder_mask_feature_source_primitive_index".to_string(),
                json!(feature.source_primitive_index),
            );
        }
        CopperObjectRef::Segment { segment, index } => {
            finding
                .measured
                .insert("solder_mask_segment_index".to_string(), json!(index));
            finding.measured.insert(
                "solder_mask_segment_start".to_string(),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                "solder_mask_segment_end".to_string(),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding.measured.insert(
                "solder_mask_segment_layer".to_string(),
                json!(segment.layer),
            );
            insert_optional_artwork_segment_owner_measurements(
                finding,
                "solder_mask_segment",
                segment,
            );
            finding.measured.insert(
                "solder_mask_segment_aperture".to_string(),
                json!(segment.aperture),
            );
            finding.measured.insert(
                "solder_mask_segment_width_mm".to_string(),
                json!(segment.width_mm),
            );
            finding.measured.insert(
                "solder_mask_segment_source_primitive".to_string(),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                "solder_mask_segment_source_primitive_index".to_string(),
                json!(segment.source_primitive_index),
            );
        }
        CopperObjectRef::Region { region, index } => {
            finding
                .measured
                .insert("solder_mask_region_index".to_string(), json!(index));
            finding
                .measured
                .insert("solder_mask_region_layer".to_string(), json!(region.layer));
            insert_optional_artwork_region_owner_measurements(
                finding,
                "solder_mask_region",
                region,
            );
            finding.measured.insert(
                "solder_mask_region_source_primitive".to_string(),
                json!(region.source_primitive),
            );
            finding.measured.insert(
                "solder_mask_region_source_primitive_index".to_string(),
                json!(region.source_primitive_index),
            );
            finding.measured.insert(
                "solder_mask_region_point_count".to_string(),
                json!(region.points.len()),
            );
        }
    }
}

pub(super) fn insert_solder_paste_object_measurements(
    finding: &mut Finding,
    object: CopperObjectRef<'_>,
) {
    insert_prefixed_solder_paste_object_measurements(finding, "", object);
}

pub(super) fn insert_prefixed_solder_paste_object_measurements(
    finding: &mut Finding,
    prefix: &str,
    object: CopperObjectRef<'_>,
) {
    let key = |field: &str| {
        if prefix.is_empty() {
            format!("solder_paste_{field}")
        } else {
            format!("{prefix}_solder_paste_{field}")
        }
    };
    finding.measured.insert(key("kind"), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding.measured.insert(key("feature_index"), json!(index));
            finding
                .measured
                .insert(key("feature_x_mm"), json!(feature.at.x_mm));
            finding
                .measured
                .insert(key("feature_y_mm"), json!(feature.at.y_mm));
            finding
                .measured
                .insert(key("feature_layer"), json!(feature.layer));
            insert_optional_copper_feature_owner_measurements(finding, &key("feature"), feature);
            finding
                .measured
                .insert(key("feature_aperture"), json!(feature.aperture));
            finding
                .measured
                .insert(key("feature_shape"), json!(feature.shape));
            finding
                .measured
                .insert(key("feature_size_x_mm"), json!(feature.size.x_mm));
            finding
                .measured
                .insert(key("feature_size_y_mm"), json!(feature.size.y_mm));
            finding.measured.insert(
                key("feature_source_primitive"),
                json!(feature.source_primitive),
            );
            finding.measured.insert(
                key("feature_source_primitive_index"),
                json!(feature.source_primitive_index),
            );
        }
        CopperObjectRef::Segment { segment, index } => {
            finding.measured.insert(key("segment_index"), json!(index));
            finding.measured.insert(
                key("segment_start"),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                key("segment_end"),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding
                .measured
                .insert(key("segment_layer"), json!(segment.layer));
            insert_optional_artwork_segment_owner_measurements(finding, &key("segment"), segment);
            finding
                .measured
                .insert(key("segment_aperture"), json!(segment.aperture));
            finding
                .measured
                .insert(key("segment_width_mm"), json!(segment.width_mm));
            finding.measured.insert(
                key("segment_source_primitive"),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                key("segment_source_primitive_index"),
                json!(segment.source_primitive_index),
            );
        }
        CopperObjectRef::Region { region, index } => {
            finding.measured.insert(key("region_index"), json!(index));
            finding
                .measured
                .insert(key("region_layer"), json!(region.layer));
            insert_optional_artwork_region_owner_measurements(finding, &key("region"), region);
            finding.measured.insert(
                key("region_source_primitive"),
                json!(region.source_primitive),
            );
            finding.measured.insert(
                key("region_source_primitive_index"),
                json!(region.source_primitive_index),
            );
            finding
                .measured
                .insert(key("region_point_count"), json!(region.points.len()));
        }
    }
}

pub(super) fn insert_solder_mask_object_measurements(
    finding: &mut Finding,
    prefix: &str,
    object: CopperObjectRef<'_>,
) {
    finding
        .measured
        .insert(format!("{prefix}_solder_mask_kind"), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding
                .measured
                .insert(format!("{prefix}_solder_mask_feature_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_x_mm"),
                json!(feature.at.x_mm),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_y_mm"),
                json!(feature.at.y_mm),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_layer"),
                json!(feature.layer),
            );
            insert_optional_copper_feature_owner_measurements(
                finding,
                &format!("{prefix}_solder_mask_feature"),
                feature,
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_aperture"),
                json!(feature.aperture),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_shape"),
                json!(feature.shape),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_size_x_mm"),
                json!(feature.size.x_mm),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_size_y_mm"),
                json!(feature.size.y_mm),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_source_primitive"),
                json!(feature.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_feature_source_primitive_index"),
                json!(feature.source_primitive_index),
            );
        }
        CopperObjectRef::Segment { segment, index } => {
            finding
                .measured
                .insert(format!("{prefix}_solder_mask_segment_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_start"),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_end"),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_layer"),
                json!(segment.layer),
            );
            insert_optional_artwork_segment_owner_measurements(
                finding,
                &format!("{prefix}_solder_mask_segment"),
                segment,
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_aperture"),
                json!(segment.aperture),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_width_mm"),
                json!(segment.width_mm),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_source_primitive"),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_source_primitive_index"),
                json!(segment.source_primitive_index),
            );
        }
        CopperObjectRef::Region { region, index } => {
            finding
                .measured
                .insert(format!("{prefix}_solder_mask_region_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_layer"),
                json!(region.layer),
            );
            insert_optional_artwork_region_owner_measurements(
                finding,
                &format!("{prefix}_solder_mask_region"),
                region,
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_source_primitive"),
                json!(region.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_source_primitive_index"),
                json!(region.source_primitive_index),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_point_count"),
                json!(region.points.len()),
            );
        }
    }
}

fn insert_optional_artwork_segment_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    segment: &LayoutCopperSegment,
) {
    insert_optional_artwork_owner_measurements(
        finding,
        prefix,
        ArtworkOwnerMeasurements {
            net: segment.net.as_deref(),
            island_id: segment.island_id.as_deref(),
            owner_kind: segment.owner_kind.as_deref(),
            component: segment.component.as_deref(),
            pin: segment.pin.as_deref(),
            via_index: segment.via_index,
        },
    );
}

fn insert_optional_artwork_region_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    region: &LayoutCopperRegion,
) {
    insert_optional_artwork_owner_measurements(
        finding,
        prefix,
        ArtworkOwnerMeasurements {
            net: region.net.as_deref(),
            island_id: region.island_id.as_deref(),
            owner_kind: region.owner_kind.as_deref(),
            component: region.component.as_deref(),
            pin: region.pin.as_deref(),
            via_index: region.via_index,
        },
    );
}

struct ArtworkOwnerMeasurements<'a> {
    net: Option<&'a str>,
    island_id: Option<&'a str>,
    owner_kind: Option<&'a str>,
    component: Option<&'a str>,
    pin: Option<&'a str>,
    via_index: Option<usize>,
}

fn insert_optional_artwork_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    owner: ArtworkOwnerMeasurements<'_>,
) {
    if let Some(net) = owner.net {
        finding.measured.insert(format!("{prefix}_net"), json!(net));
    }
    if let Some(island_id) = owner.island_id {
        finding
            .measured
            .insert(format!("{prefix}_island_id"), json!(island_id));
    }
    if let Some(owner_kind) = owner.owner_kind {
        finding
            .measured
            .insert(format!("{prefix}_owner_kind"), json!(owner_kind));
    }
    if let Some(component) = owner.component {
        finding
            .measured
            .insert(format!("{prefix}_component"), json!(component));
    }
    if let Some(pin) = owner.pin {
        finding.measured.insert(format!("{prefix}_pin"), json!(pin));
    }
    if let Some(via_index) = owner.via_index {
        finding
            .measured
            .insert(format!("{prefix}_via_index"), json!(via_index));
    }
}
